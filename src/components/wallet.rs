/// Partially copied from `zebra/zebra-chain/src/block/merkle.rs`
mod structs;

use bridgetree::{self, BridgeTree};
use incrementalmerkletree::Position;
use std::collections::BTreeMap;
use std::convert::TryInto;

use abscissa_core::prelude::info;

use orchard::issuance::{
    auth::{IssueAuthKey, IssueValidatingKey, ZSASchnorr},
    IssueBundle, Signed,
};
use orchard::keys::{FullViewingKey, IncomingViewingKey, OutgoingViewingKey, Scope, SpendingKey};
use orchard::note::{AssetBase, AssetId, ExtractedNoteCommitment, RandomSeed, Rho};
use orchard::tree::{MerkleHashOrchard, MerklePath};
use orchard::value::NoteValue;
use orchard::{bundle::Authorized, Address, Anchor, Bundle, Note};

use crate::components::persistence::model::NoteData;
use crate::components::persistence::sqlite as notes_db;
use crate::components::wallet::structs::OrderedAddress;
use crate::components::{asset_registry, block_data, tree_state};
use diesel::prelude::*;
use diesel::SqliteConnection;
use zcash_primitives::block::BlockHash;
use zcash_protocol::consensus::BlockHeight;
use zcash_primitives::transaction::components::issuance::write_note;
use zcash_primitives::transaction::{OrchardBundle, Transaction, TxId};
use bip0039::Mnemonic;
use orchard::primitives::OrchardPrimitives;
use zip32::AccountId;
use zcash_protocol::constants;
use zcash_protocol::value::ZatBalance;

pub const MAX_CHECKPOINTS: usize = 100;
pub const NOTE_COMMITMENT_TREE_DEPTH: u8 = 32;

/// Largest usable ZIP-32 account index: account indices are always hardened in
/// derivation paths, so they are effectively 31-bit ([`zip32::AccountId`]).
pub const MAX_ACCOUNT_INDEX: u32 = (1 << 31) - 1;

#[derive(Debug, Clone)]
pub enum Error {
    NoteCommitmentTreeFull,
}

#[derive(Debug, Clone)]
pub enum BundleError {
    /// The keystore did not contain the full viewing key corresponding
    /// to the incoming viewing key that successfully decrypted a note.
    FvkNotFound(IncomingViewingKey),
}

/// Combined error type returned by [`Wallet::process_block`].
#[derive(Debug)]
pub enum SyncError {
    Bundle(BundleError),
    Diesel(diesel::result::Error),
    TreeState(String),
}

impl From<BundleError> for SyncError {
    fn from(e: BundleError) -> Self {
        SyncError::Bundle(e)
    }
}
impl From<diesel::result::Error> for SyncError {
    fn from(e: diesel::result::Error) -> Self {
        SyncError::Diesel(e)
    }
}
impl From<String> for SyncError {
    fn from(e: String) -> Self {
        SyncError::TreeState(e)
    }
}

#[derive(Debug)]
pub struct NoteSpendMetadata {
    pub note: Note,
    pub sk: SpendingKey,
    pub merkle_path: MerklePath,
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

pub struct Wallet {
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
}

impl Wallet {
    fn from_seed(seed: [u8; 64]) -> Self {
        Wallet {
            key_store: KeyStore::empty(),
            commitment_tree: BridgeTree::new(MAX_CHECKPOINTS),
            last_block_height: None,
            last_block_hash: None,
            seed,
        }
    }

    /// Construct a `Wallet` from a seed phrase and restore any persisted
    /// commitment tree / sync position from SQLite. If no `wallet_state` row
    /// exists, returns a fresh wallet.
    pub fn new(conn: &mut SqliteConnection, seed_phrase: &str) -> Self {
        let seed = <Mnemonic>::from_phrase(seed_phrase).unwrap().to_seed("");
        let mut user = Self::from_seed(seed);
        if let Err(e) = user.try_load_tree_state(conn) {
            info!("Corrupt tree state, discarding: {}", e);
            if let Err(e) = tree_state::delete_tree_state(conn) {
                info!("Failed to delete corrupt tree state: {}", e);
            }
        }
        user
    }

    /// Attempt to restore the commitment tree and sync position from SQLite.
    /// Returns `Err` if persisted state exists but is corrupt.
    fn try_load_tree_state(&mut self, conn: &mut SqliteConnection) -> Result<(), String> {
        if let Some(state) = tree_state::load_tree_state(conn)? {
            let hash_bytes: [u8; 32] = hex::decode(&state.last_block_hash)
                .ok()
                .and_then(|v| v.try_into().ok())
                .ok_or_else(|| {
                    format!(
                        "Invalid last_block_hash in saved tree state: {}",
                        state.last_block_hash
                    )
                })?;
            info!(
                "Loaded saved tree state at height {}",
                state.last_block_height
            );
            self.commitment_tree = state.commitment_tree;
            self.last_block_height = Some(BlockHeight::from_u32(state.last_block_height));
            self.last_block_hash = Some(BlockHash(hash_bytes));
        }
        Ok(())
    }

    /// Reset all persisted wallet data: in-memory tree, notes, wallet_state row,
    /// the block_data hash cache, and the chain-derived flags of the asset
    /// registry. Used by `clean` and by sync_from_height when chain/wallet
    /// divergence is detected.
    ///
    /// Both callers rescan the chain from scratch afterwards, so asset
    /// finalizations are re-learned from the issuance bundles the current chain
    /// actually contains; keeping stale flags would misreport supply state.
    /// User-authored asset labels survive the reset.
    pub fn reset(&mut self, conn: &mut SqliteConnection) {
        self.commitment_tree = BridgeTree::new(MAX_CHECKPOINTS);
        self.last_block_height = None;
        self.last_block_hash = None;
        notes_db::delete_all_notes(conn);
        tree_state::delete_tree_state(conn).expect("Failed to delete tree state");
        block_data::clear(conn);
        asset_registry::clear_finalized_flags(conn);
    }

    pub fn last_block_hash(&self) -> Option<BlockHash> {
        self.last_block_hash
    }

    pub fn last_block_height(&self) -> Option<BlockHeight> {
        self.last_block_height
    }

    pub(crate) fn select_spendable_notes(
        &mut self,
        conn: &mut SqliteConnection,
        address: Address,
        total_amount: u64,
        asset: AssetBase,
    ) -> Vec<NoteSpendMetadata> {
        let all_notes = notes_db::find_non_spent_notes(conn, address, asset);
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

    /// Derive (and register) the default address of a ZIP-32 account, failing
    /// with a human-readable message for out-of-range indices instead of
    /// silently truncating them to another account or panicking.
    pub fn try_address_for_account(
        &mut self,
        account: usize,
        scope: Scope,
    ) -> Result<Address, String> {
        let account_id = u32::try_from(account)
            .ok()
            .and_then(|i| AccountId::try_from(i).ok())
            .ok_or_else(|| {
                format!(
                    "account index {} is out of range: ZIP-32 accounts are hardened, \
                     so the largest valid index is {}",
                    account, MAX_ACCOUNT_INDEX
                )
            })?;
        let sk = SpendingKey::from_zip32_seed(
            self.seed.as_slice(),
            constants::regtest::COIN_TYPE,
            account_id,
        )
        .map_err(|e| format!("could not derive keys for account {}: {}", account, e))?;
        let fvk = FullViewingKey::from(&sk);
        let address = fvk.address_at(0u32, scope);
        self.key_store.add_raw_address(address, fvk.to_ivk(scope));
        // `add_spending_key` registers the FVK (and both scopes' IVKs) it
        // derives from `sk`, so there is no separate FVK insert to make here.
        self.key_store.add_spending_key(sk);
        Ok(address)
    }

    /// [`Self::try_address_for_account`] for callers deriving a fixed, known
    /// account index (test scenarios, internal dummy recipients). Code paths
    /// that derive an index from user input must use the fallible form.
    pub fn address_for_account(&mut self, account: usize, scope: Scope) -> Address {
        self.try_address_for_account(account, scope)
            .unwrap_or_else(|e| panic!("{}", e))
    }

    /// Derive and register keys and default addresses for accounts
    /// `0..num_accounts`, so block sync can decrypt notes addressed to them
    /// and spends can locate their spending keys. Must be called before
    /// syncing when the wallet is constructed fresh (keys live in memory).
    pub fn register_accounts(&mut self, num_accounts: usize) -> Result<(), String> {
        // Check the whole range up front: deriving keys one index at a time
        // would take effectively forever to reach an out-of-range one.
        if num_accounts > MAX_ACCOUNT_INDEX as usize + 1 {
            return Err(format!(
                "{} accounts requested, but ZIP-32 account indices stop at {}",
                num_accounts, MAX_ACCOUNT_INDEX
            ));
        }
        for account in 0..num_accounts {
            self.try_address_for_account(account, Scope::External)?;
        }
        Ok(())
    }

    /// The `AssetBase` of an asset defined by this wallet's issuance key and
    /// the given asset description hash (ZIP 227 derivation).
    pub fn asset_base_from_desc_hash(&self, asset_desc_hash: &[u8; 32]) -> AssetBase {
        AssetBase::custom(&AssetId::new_v0(
            &IssueValidatingKey::from(&self.issuance_key()),
            asset_desc_hash,
        ))
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

    pub(crate) fn issuance_key(&self) -> IssueAuthKey<ZSASchnorr> {
        IssueAuthKey::from_zip32_seed(self.seed.as_slice(), constants::regtest::COIN_TYPE, 0)
            .unwrap()
    }

    pub fn balance_zec(&self, conn: &mut SqliteConnection, address: Address) -> u64 {
        self.balance(conn, address, AssetBase::zatoshi())
    }

    pub fn balance(&self, conn: &mut SqliteConnection, address: Address, asset: AssetBase) -> u64 {
        notes_db::find_non_spent_notes(conn, address, asset)
            .iter()
            .map(|n| n.amount)
            .sum::<i64>() as u64
    }

    /// Atomic per-block sync step.
    ///
    /// Wraps `block_data` insert + per-tx note inserts + tree-state save in a
    /// single SQL transaction. On error the transaction rolls back and the
    /// in-memory tree / `last_block_*` are restored from a snapshot taken on
    /// entry, keeping memory and disk consistent.
    pub fn process_block(
        &mut self,
        conn: &mut SqliteConnection,
        block_height: BlockHeight,
        block_hash: BlockHash,
        transactions: Vec<Transaction>,
    ) -> Result<(), SyncError> {
        let height_u32 = u32::from(block_height);
        let hash_hex = hex::encode(block_hash.0);

        let saved_tree = self.commitment_tree.clone();
        let saved_height = self.last_block_height;
        let saved_hash = self.last_block_hash;

        let result: Result<(), SyncError> = conn.transaction(|c| {
            block_data::insert(c, height_u32, hash_hex.clone());
            for tx in &transactions {
                if tx.version().has_orchard() || tx.version().has_orchard_zsa() {
                    self.add_notes_from_tx(c, tx)?;
                }
            }
            self.last_block_height = Some(block_height);
            self.last_block_hash = Some(block_hash);
            tree_state::save_tree_state(c, &self.commitment_tree, height_u32, &hash_hex)?;
            Ok(())
        });

        if result.is_err() {
            self.commitment_tree = saved_tree;
            self.last_block_height = saved_height;
            self.last_block_hash = saved_hash;
        }
        result
    }

    fn add_notes_from_tx(
        &mut self,
        conn: &mut SqliteConnection,
        tx: &Transaction,
    ) -> Result<(), BundleError> {
        let mut issued_notes_offset = 0;

        if let Some(orchard_bundle) = tx.orchard_bundle() {
            match orchard_bundle {
                OrchardBundle::OrchardVanilla(b) => {
                    issued_notes_offset = b.actions().len();
                    self.add_notes_from_orchard_bundle(conn, &tx.txid(), b);
                    self.mark_potential_spends(conn, &tx.txid(), b);
                }
                OrchardBundle::OrchardZSA(b) => {
                    issued_notes_offset = b.actions().len();
                    self.add_notes_from_orchard_bundle(conn, &tx.txid(), b);
                    self.mark_potential_spends(conn, &tx.txid(), b);
                }
            }
        };

        if let Some(issue_bundle) = tx.issue_bundle() {
            self.add_notes_from_issue_bundle(conn, &tx.txid(), issue_bundle, issued_notes_offset);
            self.record_finalizations(conn, issue_bundle);
        };

        self.add_note_commitments(conn, &tx.txid(), tx.orchard_bundle(), tx.issue_bundle())
            .unwrap();

        Ok(())
    }

    fn add_notes_from_orchard_bundle<O: OrchardPrimitives>(
        &mut self,
        conn: &mut SqliteConnection,
        txid: &TxId,
        bundle: &Bundle<Authorized, ZatBalance, O>,
    ) {
        let keys = self
            .key_store
            .viewing_keys
            .keys()
            .cloned()
            .collect::<Vec<_>>();

        for (action_idx, ivk, note, _recipient, memo) in bundle.decrypt_outputs_with_keys(&keys) {
            info!("Store note");
            self.store_note(conn, txid, action_idx, ivk.clone(), note, memo)
                .unwrap();
        }
    }

    fn add_notes_from_issue_bundle(
        &mut self,
        conn: &mut SqliteConnection,
        txid: &TxId,
        bundle: &IssueBundle<Signed>,
        note_index_offset: usize,
    ) {
        for (note_index, note) in bundle.actions().iter().flat_map(|a| a.notes()).enumerate() {
            if let Some(ivk) = self.key_store.ivk_for_address(&note.recipient()) {
                let note_index = note_index + note_index_offset;
                self.store_note(conn, txid, note_index, ivk.clone(), *note, [0; 512])
                    .unwrap();
            }
        }
    }

    /// Asset finalization is public data carried by issuance bundles: record
    /// it in the local asset registry so every wallet (not only the issuer)
    /// knows when an asset's supply has been closed.
    fn record_finalizations(&self, conn: &mut SqliteConnection, bundle: &IssueBundle<Signed>) {
        for action in bundle.actions().iter().filter(|a| a.is_finalized()) {
            let asset = AssetBase::custom(&AssetId::new_v0(bundle.ik(), action.asset_desc_hash()));
            asset_registry::record_seen_asset(conn, &asset);
            asset_registry::set_finalized(conn, &asset);
        }
    }

    fn store_note(
        &mut self,
        conn: &mut SqliteConnection,
        txid: &TxId,
        action_index: usize,
        ivk: IncomingViewingKey,
        note: Note,
        memo_bytes: [u8; 512],
    ) -> Result<(), BundleError> {
        if let Some(fvk) = self.key_store.viewing_keys.get(&ivk) {
            info!("Adding decrypted note to the user");

            let mut note_bytes = vec![];
            write_note(&mut note_bytes, &note).unwrap();

            let recipient = note.recipient();
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
            notes_db::insert_note(conn, note_data);

            self.key_store.add_raw_address(recipient, ivk.clone());
            Ok(())
        } else {
            info!("Can't add decrypted note, missing FVK");
            Err(BundleError::FvkNotFound(ivk.clone()))
        }
    }

    fn mark_potential_spends<O: OrchardPrimitives>(
        &mut self,
        conn: &mut SqliteConnection,
        txid: &TxId,
        orchard_bundle: &Bundle<Authorized, ZatBalance, O>,
    ) {
        for (action_index, action) in orchard_bundle.actions().iter().enumerate() {
            if let Some(note) = notes_db::find_by_nullifier(conn, action.nullifier()) {
                info!("Adding spend of nullifier {:?}", action.nullifier());
                notes_db::mark_as_potentially_spent(conn, note.id, txid, action_index as i32);
            }
        }
    }

    fn add_note_commitments(
        &mut self,
        conn: &mut SqliteConnection,
        txid: &TxId,
        orchard_bundle_opt: Option<&OrchardBundle<Authorized>>,
        issue_bundle_opt: Option<&IssueBundle<Signed>>,
    ) -> Result<(), Error> {
        let my_notes_for_tx: Vec<NoteData> = notes_db::find_notes_for_tx(conn, txid);

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
            if !self
                .commitment_tree
                .append(MerkleHashOrchard::from_cmx(commitment))
            {
                return Err(Error::NoteCommitmentTreeFull);
            }

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
                notes_db::update_note_position(conn, note.id, position as i64);
            }
        }

        Ok(())
    }
}
