/// Partially copied from `zebra/zebra-chain/src/block/merkle.rs`
mod structs;

use bridgetree::{self, BridgeTree};
use incrementalmerkletree::Position;
use std::collections::BTreeMap;
use std::convert::{TryFrom, TryInto};

use abscissa_core::prelude::info;

use zcash_primitives::{constants, legacy, transaction::components::Amount};

use orchard::domain::OrchardDomainCommon;
use orchard::issuance::{IssueBundle, Signed};
use orchard::keys::{
    FullViewingKey, IncomingViewingKey, IssuanceAuthorizingKey, OutgoingViewingKey, Scope,
    SpendingKey,
};
use orchard::note::{AssetBase, ExtractedNoteCommitment, RandomSeed, Rho};
use orchard::tree::{MerkleHashOrchard, MerklePath};
use orchard::value::NoteValue;
use orchard::{bundle::Authorized, Address, Anchor, Bundle, Note};
use rand::Rng;
use ripemd::{Digest, Ripemd160};
use secp256k1::{Secp256k1, SecretKey};
use sha2::Sha256;

use crate::components::persistence::model::NoteData;
use crate::components::persistence::sqlite::SqliteDataStorage;
use crate::components::user::structs::OrderedAddress;
use zcash_primitives::block::BlockHash;
use zcash_primitives::consensus::{BlockHeight, REGTEST_NETWORK};
use zcash_primitives::legacy::keys::NonHardenedChildIndex;
use zcash_primitives::legacy::TransparentAddress;
use zcash_primitives::transaction::components::issuance::write_note;
use zcash_primitives::transaction::{OrchardBundle, Transaction, TxId};
use zcash_primitives::zip32::AccountId;
use zcash_primitives::zip339::Mnemonic;

pub const MAX_CHECKPOINTS: usize = 100;
pub const NOTE_COMMITMENT_TREE_DEPTH: u8 = 32;

#[derive(Debug, Clone)]
pub enum WalletError {
    OutOfOrder(BlockHeight, usize),
    NoteCommitmentTreeFull,
}

#[derive(Debug, Clone)]
pub enum BundleLoadError {
    /// The action at the specified index failed to decrypt with
    /// the provided IVK.
    ActionDecryptionFailed(usize),
    /// The keystore did not contain the full viewing key corresponding
    /// to the incoming viewing key that successfully decrypted a
    /// note.
    FvkNotFound(IncomingViewingKey),
    /// An action index identified as potentially spending one of our
    /// notes is not a valid action index for the bundle.
    InvalidActionIndex(usize),
    /// Invalid Transaction data format
    InvalidTransactionFormat,
}

#[derive(Debug)]
pub struct NoteSpendMetadata {
    pub note: Note,
    pub sk: SpendingKey,
    pub merkle_path: MerklePath,
}

struct KeyStore {
    accounts: BTreeMap<u32, Address>,
    payment_addresses: BTreeMap<OrderedAddress, IncomingViewingKey>,
    viewing_keys: BTreeMap<IncomingViewingKey, FullViewingKey>,
    spending_keys: BTreeMap<FullViewingKey, SpendingKey>,
}

impl KeyStore {
    pub fn empty() -> Self {
        KeyStore {
            accounts: BTreeMap::new(),
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

    /// Adds an address/ivk pair and returns `true` if the IVK
    /// corresponds to a known FVK, `false` otherwise.
    pub fn add_raw_address(&mut self, addr: Address, ivk: IncomingViewingKey) -> bool {
        let has_fvk = self.viewing_keys.contains_key(&ivk);
        self.payment_addresses
            .insert(OrderedAddress::new(addr), ivk);
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

pub struct User {
    /// The database used to store the user's state.
    db: SqliteDataStorage,
    /// The in-memory index of keys and addresses known to the user.
    key_store: KeyStore,
    /// The incremental Merkle tree used to track note commitments and witnesses for notes
    /// belonging to the user.
    commitment_tree: BridgeTree<MerkleHashOrchard, u32, NOTE_COMMITMENT_TREE_DEPTH>,
    /// The block height at which the last checkpoint was created, if any.
    last_block_height: Option<BlockHeight>,
    /// The block hash at which the last checkpoint was created, if any.
    last_block_hash: Option<BlockHash>,
    /// The seed used to derive the user's keys.
    seed: [u8; 64],
    /// The seed used to derive the miner's keys. This is a hack for claiming coinbase.
    miner_seed: [u8; 64],
}

impl User {
    pub fn new(seed_phrase: &String, miner_seed_phrase: &String) -> Self {
        User {
            db: SqliteDataStorage::new(),
            key_store: KeyStore::empty(),
            commitment_tree: BridgeTree::new(MAX_CHECKPOINTS),
            last_block_height: None,
            last_block_hash: None,
            seed: Mnemonic::from_phrase(seed_phrase).unwrap().to_seed(""),
            miner_seed: Mnemonic::from_phrase(miner_seed_phrase)
                .unwrap()
                .to_seed(""),
        }
    }

    pub fn random(miner_seed_phrase: &String) -> Self {
        let mut rng = rand::thread_rng();
        let mut seed_random_bytes = [0u8; 64];
        rng.fill(&mut seed_random_bytes);

        User {
            db: SqliteDataStorage::new(),
            key_store: KeyStore::empty(),
            commitment_tree: BridgeTree::new(MAX_CHECKPOINTS),
            last_block_height: None,
            last_block_hash: None,
            seed: seed_random_bytes,
            miner_seed: Mnemonic::from_phrase(miner_seed_phrase)
                .unwrap()
                .to_seed(""),
        }
    }

    /// Reset the state to be suitable for rescan from the NU5 activation
    /// height.  This removes all witness and spentness information from the user. The
    /// keystore is unmodified and decrypted note, nullifier, and conflict data are left
    /// in place with the expectation that they will be overwritten and/or updated in
    /// the rescan process.
    pub fn reset(&mut self) {
        self.commitment_tree = BridgeTree::new(MAX_CHECKPOINTS);
        self.last_block_height = None;
        self.last_block_hash = None;
        self.db.delete_all_notes();
    }

    pub fn last_block_hash(&self) -> Option<BlockHash> {
        self.last_block_hash
    }

    pub fn last_block_height(&self) -> Option<BlockHeight> {
        self.last_block_height
    }

    pub(crate) fn select_spendable_notes(
        &mut self,
        address: Address,
        total_amount: u64,
        asset: AssetBase,
    ) -> Vec<NoteSpendMetadata> {
        let all_notes = self.db.find_non_spent_notes(address, asset);
        let mut selected_notes = Vec::new();
        let mut total_amount_selected = 0;

        for note_data in all_notes {
            let rho = Rho::from_bytes(note_data.rho.as_slice().try_into().unwrap()).unwrap();
            let note = Note::from_parts(
                Address::from_raw_address_bytes(
                    note_data.recipient_address.as_slice().try_into().unwrap(),
                )
                .unwrap(),
                NoteValue::from_raw(note_data.amount as u64),
                AssetBase::from_bytes(note_data.asset.as_slice().try_into().unwrap()).unwrap(),
                rho,
                RandomSeed::from_bytes(note_data.rseed.as_slice().try_into().unwrap(), &rho)
                    .unwrap(),
            )
            .unwrap();

            let note_value = note.value().inner();
            let sk = self
                .key_store
                .spending_key_for_ivk(
                    self.key_store
                        .ivk_for_address(&note.recipient())
                        .expect("IVK not found for address"),
                )
                .expect("SpendingKey not found for IVK");

            let merkle_path = MerklePath::from_parts(
                note_data.position as u32,
                self.commitment_tree
                    .witness(Position::from(note_data.position as u64), 0)
                    .unwrap()
                    .try_into()
                    .unwrap(),
            );

            selected_notes.push(NoteSpendMetadata {
                note,
                sk: *sk,
                merkle_path,
            });
            total_amount_selected += note_value;

            if total_amount_selected >= total_amount {
                break;
            }
        }
        if total_amount_selected < total_amount {
            panic!(
                "insufficient inputs: required {} but found {}",
                total_amount, total_amount_selected
            );
        }

        selected_notes
    }

    pub fn address_for_account(&mut self, account: usize, scope: Scope) -> Address {
        let account = account as u32;
        match self.key_store.accounts.get(&(account)) {
            Some(addr) => *addr,
            None => {
                let sk = SpendingKey::from_zip32_seed(
                    self.seed.as_slice(),
                    constants::regtest::COIN_TYPE,
                    AccountId::try_from(account).unwrap(),
                )
                .unwrap();
                let fvk = FullViewingKey::from(&sk);
                let address = fvk.address_at(0u32, scope);
                self.key_store.add_raw_address(address, fvk.to_ivk(scope));
                self.key_store.add_full_viewing_key(fvk);
                self.key_store.add_spending_key(sk);
                self.key_store.accounts.insert(account, address);
                address
            }
        }
    }

    pub(crate) fn orchard_ovk(&self) -> OutgoingViewingKey {
        let sk = SpendingKey::from_zip32_seed(
            self.seed.as_slice(),
            constants::regtest::COIN_TYPE,
            AccountId::try_from(0).unwrap(),
        )
        .unwrap();
        FullViewingKey::from(&sk).to_ovk(Scope::External)
    }

    pub(crate) fn orchard_anchor(&self) -> Option<Anchor> {
        Some(Anchor::from(self.commitment_tree.root(0).unwrap()))
    }

    pub(crate) fn issuance_key(&self) -> IssuanceAuthorizingKey {
        IssuanceAuthorizingKey::from_zip32_seed(
            self.seed.as_slice(),
            constants::testnet::COIN_TYPE,
            0,
        )
        .unwrap()
    }

    // Hack for claiming coinbase
    pub fn miner_address(&self) -> TransparentAddress {
        let account = AccountId::try_from(0).unwrap();
        let pubkey =
            legacy::keys::AccountPrivKey::from_seed(&REGTEST_NETWORK, &self.miner_seed, account)
                .unwrap()
                .derive_external_secret_key(NonHardenedChildIndex::ZERO)
                .unwrap()
                .public_key(&Secp256k1::new())
                .serialize();
        let hash = &Ripemd160::digest(Sha256::digest(pubkey))[..];
        TransparentAddress::PublicKeyHash(hash.try_into().unwrap())
    }

    // Hack for claiming coinbase
    pub fn miner_sk(&self) -> SecretKey {
        let account = AccountId::try_from(0).unwrap();
        legacy::keys::AccountPrivKey::from_seed(&REGTEST_NETWORK, &self.miner_seed, account)
            .unwrap()
            .derive_external_secret_key(NonHardenedChildIndex::ZERO)
            .unwrap()
    }

    pub fn balance_zec(&mut self, address: Address) -> u64 {
        self.balance(address, AssetBase::native())
    }

    pub fn balance(&mut self, address: Address, asset: AssetBase) -> u64 {
        let all_notes = self.db.find_non_spent_notes(address, asset);
        let mut total_amount: i64 = 0;

        for note_data in all_notes {
            total_amount += note_data.amount;
        }
        total_amount as u64
    }

    /// Add note data from all V5 transactions of the block to the user.
    /// Versions other than V5 are ignored.
    pub fn add_notes_from_block(
        &mut self,
        block_height: BlockHeight,
        block_hash: BlockHash,
        transactions: Vec<Transaction>,
    ) -> Result<(), BundleLoadError> {
        transactions.into_iter().try_for_each(|tx| {
            if tx.version().has_orchard() || tx.version().has_orchard_zsa() {
                self.add_notes_from_tx(tx)?;
            };
            Ok(())
        })?;

        self.last_block_hash = Some(block_hash);
        self.last_block_height = Some(block_height);
        Ok(())
    }

    /// Add note data to the user, and return a data structure that describes
    /// the actions that are involved with this user.
    pub fn add_notes_from_tx(&mut self, tx: Transaction) -> Result<(), BundleLoadError> {
        let mut issued_notes_offset = 0;

        if let Some(orchard_bundle) = tx.orchard_bundle() {
            // Add notes from Orchard bundle
            match orchard_bundle {
                OrchardBundle::OrchardVanilla(b) => {
                    issued_notes_offset = b.actions().len();
                    self.add_notes_from_orchard_bundle(&tx.txid(), b);
                    self.mark_potential_spends(&tx.txid(), b);
                }
                OrchardBundle::OrchardZSA(b) => {
                    issued_notes_offset = b.actions().len();
                    self.add_notes_from_orchard_bundle(&tx.txid(), b);
                    self.mark_potential_spends(&tx.txid(), b);
                }
            }
        };

        // Add notes from Issue bundle
        if let Some(issue_bundle) = tx.issue_bundle() {
            self.add_notes_from_issue_bundle(&tx.txid(), issue_bundle, issued_notes_offset);
        };

        self.add_note_commitments(&tx.txid(), tx.orchard_bundle(), tx.issue_bundle())
            .unwrap();

        Ok(())
    }

    /// Add note data for those notes that are decryptable with one of this user's
    /// incoming viewing keys, and return a data structure that describes
    /// the actions that are involved with this user, either spending notes belonging
    /// to this user or creating new notes owned by this user.
    fn add_notes_from_orchard_bundle<O: OrchardDomainCommon>(
        &mut self,
        txid: &TxId,
        bundle: &Bundle<Authorized, Amount, O>,
    ) {
        let keys = self
            .key_store
            .viewing_keys
            .keys()
            .cloned()
            .collect::<Vec<_>>();

        for (action_idx, ivk, note, recipient, memo) in bundle.decrypt_outputs_with_keys(&keys) {
            info!("Store note");
            self.store_note(txid, action_idx, ivk.clone(), note, recipient, memo)
                .unwrap();
        }
    }

    /// Add note data to the user, and return a data structure that describes
    /// the actions that are involved with this user.
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
                )
                .unwrap();
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
            info!("Adding decrypted note to the user");

            let mut note_bytes = vec![];
            write_note(&mut note_bytes, &note).unwrap();

            let note_data = NoteData {
                id: 0,
                amount: note.value().inner() as i64,
                asset: note.asset().to_bytes().to_vec(),
                tx_id: txid.as_ref().to_vec(),
                action_index: action_index as i32,
                position: -1,
                memo: memo_bytes.to_vec(),
                rho: note.rho().to_bytes().to_vec(),
                nullifier: note.nullifier(fvk).to_bytes().to_vec(),
                rseed: note.rseed().as_bytes().to_vec(),
                recipient_address: recipient.to_raw_address_bytes().to_vec(),
                spend_tx_id: None,
                spend_action_index: -1,
            };
            self.db.insert_note(note_data);

            // add the association between the address and the IVK used
            // to decrypt the note
            self.key_store.add_raw_address(recipient, ivk.clone());
            Ok(())
        } else {
            info!("Can't add decrypted note, missing FVK");
            Err(BundleLoadError::FvkNotFound(ivk.clone()))
        }
    }

    fn mark_potential_spends<O: OrchardDomainCommon>(
        &mut self,
        txid: &TxId,
        orchard_bundle: &Bundle<Authorized, Amount, O>,
    ) {
        for (action_index, action) in orchard_bundle.actions().iter().enumerate() {
            if let Some(note) = self.db.find_by_nullifier(action.nullifier()) {
                info!("Adding spend of nullifier {:?}", action.nullifier());
                self.db
                    .mark_as_potentially_spent(note.id, txid, action_index as i32);
            }
        }
    }

    /// Add note commitments for the Orchard components of a transaction to the note
    /// commitment tree, and mark the tree at the notes decryptable by this user so that
    /// in the future we can produce authentication paths to those notes.
    pub fn add_note_commitments(
        &mut self,
        txid: &TxId,
        orchard_bundle_opt: Option<&OrchardBundle<Authorized>>,
        issue_bundle_opt: Option<&IssueBundle<Signed>>,
    ) -> Result<(), WalletError> {
        let my_notes_for_tx: Vec<NoteData> = self.db.find_notes_for_tx(txid);

        // Process note commitments
        let mut note_commitments: Vec<ExtractedNoteCommitment> =
            if let Some(bundle) = orchard_bundle_opt {
                match bundle {
                    OrchardBundle::OrchardVanilla(b) => {
                        b.actions().iter().map(|action| *action.cmx()).collect()
                    }
                    OrchardBundle::OrchardZSA(b) => {
                        b.actions().iter().map(|action| *action.cmx()).collect()
                    }
                }
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
            info!("Adding note commitment ({}, {})", txid, note_index);
            // append the note commitment for each action to the note commitment tree
            if !self
                .commitment_tree
                .append(MerkleHashOrchard::from_cmx(commitment))
            {
                return Err(WalletError::NoteCommitmentTreeFull);
            }

            // for notes that are ours, mark the current state of the tree
            if let Some(note) = my_notes_for_tx
                .iter()
                .find(|note| note.action_index == note_index as i32)
            {
                info!("Witnessing Orchard note ({}, {})", txid, note_index);
                let position: u64 = self
                    .commitment_tree
                    .mark()
                    .expect("tree is not empty")
                    .into();
                self.db.update_note_position(note.id, position as i64);
            }
        }

        Ok(())
    }
}
