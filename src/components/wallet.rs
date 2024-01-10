mod structs;

use bridgetree::{self, BridgeTree};
use incrementalmerkletree::Position;
use std::collections::BTreeMap;
use std::convert::TryInto;
use abscissa_core::component::AsAny;
use abscissa_core::prelude::{error, info};

use zcash_primitives::{constants, legacy, sapling::NOTE_COMMITMENT_TREE_DEPTH, transaction::{components::Amount}};

use orchard::{bundle::Authorized, Address, Bundle, Note, Anchor};
use orchard::issuance::{IssueBundle, Signed};
use orchard::keys::{IssuanceAuthorizingKey, OutgoingViewingKey, FullViewingKey, IncomingViewingKey, Scope, SpendingKey, PreparedIncomingViewingKey};
use orchard::note::ExtractedNoteCommitment;
use orchard::note_encryption_v3::OrchardDomainV3;
use orchard::tree::{MerklePath, MerkleHashOrchard};
use ripemd::{Digest, Ripemd160};
use secp256k1::{Secp256k1, SecretKey};
use sha2::{ Sha256, Digest as Sha2Digest };
use zcash_client_backend::encoding::AddressCodec;

use zcash_note_encryption::{ShieldedOutput, try_note_decryption};
use zcash_primitives::block::BlockHash;
use zcash_primitives::consensus::{BlockHeight, TEST_NETWORK};
use zcash_primitives::legacy::TransparentAddress;
use zcash_primitives::transaction::components::issuance::{read_note, write_note};
use zcash_primitives::transaction::{Transaction, TxId};
use zcash_primitives::zip32::AccountId;
use crate::components::persistence::model::NoteData;
use crate::components::persistence::sqlite::SqliteDataStorage;
use crate::components::wallet::structs::OrderedAddress;

pub const MAX_CHECKPOINTS: usize = 100;

#[derive(Debug)]
pub struct NoteSpendMetadata {
    pub note: Note,
    pub sk: SpendingKey,
    pub merkle_path: MerklePath
}

struct KeyStore {
    payment_addresses: BTreeMap<OrderedAddress, IncomingViewingKey>,
    viewing_keys: BTreeMap<IncomingViewingKey, FullViewingKey>,
    spending_keys: BTreeMap<FullViewingKey, SpendingKey>,
}

impl KeyStore {
    pub fn empty() -> Self {
        KeyStore {
            payment_addresses: BTreeMap::new(),
            viewing_keys: BTreeMap::new(),
            spending_keys: BTreeMap::new(),
        }
    }

    pub fn add_full_viewing_key(&mut self, fvk: FullViewingKey) {
        // When we add a full viewing key, we need to add both the internal and external
        // incoming viewing keys.
        let external_ivk = fvk.to_ivk(Scope::External);
        let internal_ivk = fvk.to_ivk(Scope::Internal);
        self.viewing_keys.insert(external_ivk, fvk.clone());
        self.viewing_keys.insert(internal_ivk, fvk);
    }

    pub fn add_spending_key(&mut self, sk: SpendingKey) {
        let fvk = FullViewingKey::from(&sk);
        self.add_full_viewing_key(fvk.clone());
        self.spending_keys.insert(fvk, sk);
    }

    /// Adds an address/ivk pair to the wallet, and returns `true` if the IVK
    /// corresponds to a FVK known by this wallet, `false` otherwise.
    pub fn add_raw_address(&mut self, addr: Address, ivk: IncomingViewingKey) -> bool {
        let has_fvk = self.viewing_keys.contains_key(&ivk);
        self.payment_addresses.insert(OrderedAddress::new(addr), ivk);
        has_fvk
    }

    pub fn spending_key_for_ivk(&self, ivk: &IncomingViewingKey) -> Option<&SpendingKey> {
        self.viewing_keys
            .get(ivk)
            .and_then(|fvk| self.spending_keys.get(fvk))
    }

    pub fn ivk_for_address(&self, addr: &Address) -> Option<&IncomingViewingKey> {
        self.payment_addresses.get(&OrderedAddress::new(*addr))
    }
}

pub struct Wallet {
    db: SqliteDataStorage,
    /// The in-memory index of keys and addresses known to the wallet.
    key_store: KeyStore,
    /// The incremental Merkle tree used to track note commitments and witnesses for notes
    /// belonging to the wallet.
    commitment_tree: BridgeTree<MerkleHashOrchard, u32, NOTE_COMMITMENT_TREE_DEPTH>,
    /// The block height at which the last checkpoint was created, if any.
    last_block_height: Option<BlockHeight>,
    /// The block hash at which the last checkpoint was created, if any.
    last_block_hash: Option<BlockHash>,
    /// The seed used to derive the wallet's keys. TODO better seed handling
    seed: [u8; 32]
}

impl Wallet {

    pub fn last_block_hash(&self) -> Option<BlockHash> {
        self.last_block_hash
    }

    pub fn last_block_height(&self) -> Option<BlockHeight> {
        self.last_block_height
    }

    pub(crate) fn reorg(&self, height: BlockHeight) {
        todo!()
    }

    pub(crate) fn select_spendable_notes(&mut self, total_amount: u64) -> Vec<NoteSpendMetadata> {

        // TODO implement more sophisticated and efficient note selection logic

        let all_notes = self.db.find_notes();
        let mut selected_notes = Vec::new();
        let mut total_amount_selected = 0;

        for note_data in all_notes {
            let note = read_note(&note_data.serialized_note[..]).unwrap();
            let note_value = note.value().inner();
            let sk = self.key_store.spending_key_for_ivk(self.key_store.ivk_for_address(&note.recipient()).expect("IVK not found for address")).expect("SpendingKey not found for IVK");

            let merkle_path = MerklePath::from_parts(note_data.position as u32, self.commitment_tree.witness(Position::from(note_data.position as u64), 0).unwrap().try_into().unwrap());

            selected_notes.push(NoteSpendMetadata {
                note,
                sk: sk.clone(),
                merkle_path,
            });
            total_amount_selected += note_value;

            if total_amount_selected + note_value >= total_amount { break }
        };
        selected_notes
    }

    pub fn address_for_account(&mut self, account: u32, scope: Scope) -> Address {
        let sk = SpendingKey::from_zip32_seed(self.seed.as_slice(), constants::testnet::COIN_TYPE, account).unwrap();
        let fvk = FullViewingKey::from(&sk);
        let address = fvk.address_at(0u32, scope);
        self.key_store.add_raw_address(address, fvk.to_ivk(scope));
        self.key_store.add_full_viewing_key(fvk);
        self.key_store.add_spending_key(sk);
        address
    }

    pub(crate) fn change_address(&mut self) -> Address {
        self.address_for_account(0, Scope::Internal)
    }

    pub(crate) fn orchard_ovk(&self) -> OutgoingViewingKey {
        let sk = SpendingKey::from_zip32_seed(self.seed.as_slice(), constants::testnet::COIN_TYPE, 0).unwrap();
        FullViewingKey::from(&sk).to_ovk(Scope::External)
    }

    pub(crate) fn orchard_anchor(&self) -> Option<Anchor> {
        Some(Anchor::from(self.commitment_tree.root(0).unwrap()))
    }

    pub(crate) fn issuance_key(&self) -> IssuanceAuthorizingKey {
        IssuanceAuthorizingKey::from_zip32_seed(self.seed.as_slice(), constants::testnet::COIN_TYPE, 0).unwrap()
    }

    // Hack for claiming coinbase
    pub fn miner_address(&mut self) -> TransparentAddress {
        let seed: [u8; 32] = [0; 32];
        let account = AccountId::from(0);
        let pubkey = legacy::keys::AccountPrivKey::from_seed(&TEST_NETWORK, &seed, account).unwrap().derive_external_secret_key(0).unwrap().public_key(&Secp256k1::new()).serialize();
        let hash = &Ripemd160::digest(Sha256::digest(pubkey))[..];
        let taddr = TransparentAddress::PublicKey(hash.try_into().unwrap());
        taddr // "tmCUAhdeyFwVxETt3xTZrrQ3Z87e1Grakc1"
    }

    pub fn miner_sk(&mut self) -> SecretKey {
        let seed: [u8; 32] = [0; 32];
        let account = AccountId::from(0);
        legacy::keys::AccountPrivKey::from_seed(&TEST_NETWORK, &seed, account).unwrap().derive_external_secret_key(0).unwrap()
    }
}

#[derive(Debug, Clone)]
pub enum WalletError {
    OutOfOrder(BlockHeight, usize),
    NoteCommitmentTreeFull
}

#[derive(Debug, Clone)]
pub enum RewindError {
    /// The note commitment tree does not contain enough checkpoints to
    /// rewind to the requested height. The number of blocks that
    /// it is possible to rewind is returned as the payload of
    /// this error.
    InsufficientCheckpoints(usize),
}

#[derive(Debug, Clone)]
pub enum BundleLoadError {
    /// The action at the specified index failed to decrypt with
    /// the provided IVK.
    ActionDecryptionFailed(usize),
    /// The wallet did not contain the full viewing key corresponding
    /// to the incoming viewing key that successfully decrypted a
    /// note.
    FvkNotFound(IncomingViewingKey),
    /// An action index identified as potentially spending one of our
    /// notes is not a valid action index for the bundle.
    InvalidActionIndex(usize),
    /// Invalid Transaction data format
    InvalidTransactionFormat
}

impl Wallet {
    pub fn new() -> Self {
        Wallet {
            db: SqliteDataStorage::new(),
            key_store: KeyStore::empty(),
            commitment_tree: BridgeTree::new(MAX_CHECKPOINTS),
            last_block_height: None,
            last_block_hash: None,
            seed: [0; 32]
        }
    }

    /// Reset the state of the wallet to be suitable for rescan from the NU5 activation
    /// height.  This removes all witness and spentness information from the wallet. The
    /// keystore is unmodified and decrypted note, nullifier, and conflict data are left
    /// in place with the expectation that they will be overwritten and/or updated in
    /// the rescan process.
    pub fn reset(&mut self) {
        self.commitment_tree = BridgeTree::new(MAX_CHECKPOINTS);
        self.last_block_height = None;
        self.last_block_hash = None;
        // TODO clean keystore
        self.db.delete_all_notes();
    }

    /// Checkpoints the note commitment tree. This returns `false` and leaves the note
    /// commitment tree unmodified if the block height does not immediately succeed
    /// the last checkpointed block height (unless the note commitment tree is empty,
    /// in which case it unconditionally succeeds). This must be called exactly once
    /// per block.
    fn checkpoint(&mut self, block_height: BlockHeight) -> bool {
        // checkpoints must be in order of sequential block height and every
        // block must be checkpointed
        if let Some(last_height) = self.last_block_height {
            let expected_height = last_height + 1;

            if block_height != expected_height {
                error!(
                    "Expected checkpoint height {}, given {}",
                    expected_height,
                    block_height
                );
                return false;
            }
        }

        let block_height: u32 = block_height.into();
        self.commitment_tree.checkpoint(block_height);
        true
    }

    /// Add note data from all V5 transactions of the block to the wallet.
    /// Versions other than V5 are ignored.
    pub fn add_notes_from_block(&mut self, block_height: BlockHeight, block_hash: BlockHash, transactions: Vec<Transaction>) -> Result<(), BundleLoadError> {
        transactions.into_iter().for_each( |tx| if tx.version().has_orchard() {
            self.add_notes_from_tx(tx).unwrap();
        });

        self.checkpoint(block_height);

        self.last_block_hash = Some(block_hash);
        self.last_block_height = Some(block_height);
        Ok(())
    }

    /// Add note data to the wallet, and return a a data structure that describes
    /// the actions that are involved with this wallet.
    pub fn add_notes_from_tx(&mut self, tx: Transaction) -> Result<(), BundleLoadError> {

        let mut issued_notes_offset = 0;

         // Add note from Orchard bundle
        if let Some(orchard_bundle) = tx.orchard_bundle() {
            issued_notes_offset = orchard_bundle.actions().len();
            self.add_notes_from_orchard_bundle(&tx.txid(), orchard_bundle);
            self.mark_potential_spends(&tx.txid(), orchard_bundle);
        };

        // Add notes from Issue bundle
        if let Some(issue_bundle) = tx.issue_bundle() {
            self.add_notes_from_issue_bundle(&tx.txid(), issue_bundle, issued_notes_offset);
        };

        self.add_note_commitments(&tx.txid(), tx.orchard_bundle(), tx.issue_bundle()).unwrap();

        Ok(())
    }


    /// Add note data for those notes that are decryptable with one of this wallet's
    /// incoming viewing keys to the wallet, and return a a data structure that describes
    /// the actions that are involved with this wallet, either spending notes belonging
    /// to this wallet or creating new notes owned by this wallet.
    fn add_notes_from_orchard_bundle(
        &mut self,
        txid: &TxId,
        bundle: &Bundle<Authorized, Amount>,
    ) {
        let keys = self
            .key_store
            .viewing_keys
            .keys()
            .cloned()
            .collect::<Vec<_>>();

        for (action_idx, ivk, note, recipient, memo) in self.decrypt_outputs_with_keys(&bundle, &keys) {
            self.store_note(txid, action_idx, ivk.clone(), note, recipient, memo).unwrap();
        }
    }

    /// Performs trial decryption of each action in the bundle with each of the
    /// specified incoming viewing keys, and returns a vector of each decrypted
    /// note plaintext contents along with the index of the action from which it
    /// was derived.
    fn decrypt_outputs_with_keys(
        &self,
        bundle: &Bundle<Authorized, Amount>,
        keys: &[IncomingViewingKey],
    ) -> Vec<(usize, IncomingViewingKey, Note, Address, [u8; 512])> {
        let prepared_keys: Vec<_> = keys
            .iter()
            .map(|ivk| (ivk, PreparedIncomingViewingKey::new(ivk)))
            .collect();
        bundle.actions()
            .iter()
            .enumerate()
            .filter_map(|(idx, action)| {
                let domain = OrchardDomainV3::for_action(&action);
                prepared_keys.iter().find_map(|(ivk, prepared_ivk)| {
                    try_note_decryption(&domain, prepared_ivk, action)
                        .map(|(n, a, m)| (idx, (*ivk).clone(), n, a, m))
                })
            })
            .collect()
    }

    /// Add note data to the wallet, and return a a data structure that describes
    /// the actions that are involved with this wallet.
    fn add_notes_from_issue_bundle(
        &mut self,
        txid: &TxId,
        bundle: &IssueBundle<Signed>,
        note_index_offset: usize,
    ) {
        for (note_index, note) in bundle.actions().iter().flat_map(|a| a.notes()).enumerate() {
            if let Some(ivk) = self.key_store.ivk_for_address(&note.recipient()) {
                let note_index = note_index + note_index_offset;
                self.store_note(
                    txid,
                    note_index,
                    ivk.clone(),
                    *note,
                    note.recipient(),
                    [0; 512],
                ).unwrap();
            }
        }
    }

    fn store_note(
        &mut self,
        txid: &TxId,
        action_index: usize,
        ivk: IncomingViewingKey,
        note: Note,
        recipient: Address,
        memo_bytes: [u8; 512],
    ) -> Result<(), BundleLoadError> {
        if let Some(fvk) = self.key_store.viewing_keys.get(&ivk) {
            info!("Adding decrypted note to the wallet");

            // TODO do we want to store nullifier separately?
            let nf = note.nullifier(fvk);

            let mut note_bytes = vec![];
            write_note(&note, &mut note_bytes).unwrap();

            let note_data = NoteData {
                id: 0,
                amount: note.value().inner() as i64,
                asset: note.asset().to_bytes().to_vec(),
                tx_id: txid.as_ref().to_vec(),
                action_index: action_index as i32,
                position: -1,
                serialized_note: note_bytes,
                memo: memo_bytes.to_vec(),
                nullifier: nf.to_bytes().to_vec(),
                spend_tx_id: None,
                spend_action_index: -1
            };
            self.db.insert_note(note_data);

            // add the association between the address and the IVK used
            // to decrypt the note
            self.key_store.add_raw_address(recipient, ivk.clone());
            Ok(())
        } else {
            info!("Can't add decrypted note to the wallet, missing FVK");
            Err(BundleLoadError::FvkNotFound(ivk.clone()))
        }
    }

    fn mark_potential_spends(&mut self, txid: &TxId, orchard_bundle: &Bundle<Authorized, Amount>) {
        for (action_index, action) in orchard_bundle.actions().iter().enumerate() {
            match self.db.find_by_nullifier(action.nullifier()) {
                Some(note) => {
                    info!("Adding potential spend of nullifier {:?}", action.nullifier());
                    self.db.mark_as_potentially_spent(note.id, txid, action_index as i32);
                },
                None => {}
            }
        }
    }

    /// Add note commitments for the Orchard components of a transaction to the note
    /// commitment tree, and mark the tree at the notes decryptable by this wallet so that
    /// in the future we can produce authentication paths to those notes.
    ///
    /// * `block_height` - Height of the block containing the transaction that provided
    ///   this bundle.
    /// * `block_tx_idx` - Index of the transaction within the block
    /// * `txid` - Identifier of the transaction.
    /// * `bundle_opt` - Orchard component of the transaction (may be null for issue-only tx).
    /// * `issue_bundle_opt` - IssueBundle component of the transaction  (may be null for transfer-only tx).
    pub fn add_note_commitments(
        &mut self,
        txid: &TxId,
        orchard_bundle_opt: Option<&Bundle<Authorized, Amount>>,
        issue_bundle_opt: Option<&IssueBundle<Signed>>,
    ) -> Result<(), WalletError> {
        // TODO Check that the wallet is in the correct state to update the note commitment tree with new outputs.

        // update the block height recorded for the transaction
        let my_notes_for_tx: Vec<NoteData> = self.db.find_notes_for_tx(txid);

        // Process note commitments
        let mut note_commitments: Vec<ExtractedNoteCommitment> = if let Some(bundle) = orchard_bundle_opt {
            bundle
                .actions()
                .iter()
                .map(|action| *action.cmx())
                .collect()
        } else {
            Vec::new()
        };

        let mut issued_note_commitments: Vec<ExtractedNoteCommitment> =
            if let Some(issue_bundle) = issue_bundle_opt {
                issue_bundle
                    .actions()
                    .iter()
                    .flat_map(|a| a.notes())
                    .map(|note| note.commitment().into())
                    .collect()
            } else {
                Vec::new()
            };

        note_commitments.append(&mut issued_note_commitments);

        for (note_index, commitment) in note_commitments.iter().enumerate() {
            // append the note commitment for each action to the note commitment tree
            if !self
                .commitment_tree
                .append(MerkleHashOrchard::from_cmx(commitment))
            {
                return Err(WalletError::NoteCommitmentTreeFull);
            }

            // for notes that are ours, mark the current state of the tree
            match my_notes_for_tx.iter().find(|note| note.action_index == note_index as i32) {
                Some(note) => {
                    info!("Witnessing Orchard note ({}, {})", txid, note_index);
                    let position: u64 = self.commitment_tree.mark().expect("tree is not empty").into();
                    self.db.update_note_position(note.id, position as i64);
                }
                None => {}
            }
        }

        Ok(())
    }
}