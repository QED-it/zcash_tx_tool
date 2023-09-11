mod structs;

use bridgetree::{self, BridgeTree};
use byteorder::{ReadBytesExt, WriteBytesExt};
use incrementalmerkletree::Position;
use std::collections::{BTreeMap, BTreeSet};
use std::convert::TryInto;
use abscissa_core::prelude::{error, info};

use zcash_primitives::{
    consensus::BlockHeight,
    sapling::NOTE_COMMITMENT_TREE_DEPTH,
    transaction::{components::Amount, TxId},
};

use orchard::issuance::{IssueBundle, Signed};
use orchard::keys::IssuanceAuthorizingKey;
use orchard::note::{AssetBase, ExtractedNoteCommitment};
use orchard::{
    bundle::Authorized,
    keys::{FullViewingKey, IncomingViewingKey, Scope, SpendingKey},
    note::Nullifier,
    tree::MerkleHashOrchard,
    Address, Bundle, Note,
};
use orchard::tree::MerklePath;
use zebra_chain::block::Block;
use zebra_chain::transaction::Transaction;
use crate::components::wallet::structs::{OrderedAddress, OrchardSpendInfo};


pub const MAX_CHECKPOINTS: usize = 100;

/// A data structure tracking the last transaction whose notes
/// have been added to the wallet's note commitment tree.
#[derive(Debug, Clone)]
pub struct LastObserved {
    block_height: BlockHeight,
    block_tx_idx: Option<usize>,
}

/// A pointer to a particular action in an Orchard transaction output.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct OutPoint {
    txid: TxId,
    action_idx: usize,
}

/// A pointer to a previous output being spent in an Orchard action.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct InPoint {
    txid: TxId,
    action_idx: usize,
}

#[derive(Debug, Clone)]
pub struct DecryptedNote {
    note: Note,
    memo: [u8; 512],
}

/// A data structure tracking the note data that was decrypted from a single transaction.
#[derive(Debug, Clone)]
pub struct TxNotes {
    /// A map from the index of the Orchard action from which this note
    /// was decrypted to the decrypted note value.
    decrypted_notes: BTreeMap<usize, DecryptedNote>,
}

/// A data structure holding chain position information for a single transaction.
#[derive(Clone, Debug)]
struct NotePositions {
    /// The height of the block containing the transaction.
    tx_height: BlockHeight,
    /// A map from the index of an Orchard action tracked by this wallet, to the position
    /// of the output note's commitment within the global Merkle tree.
    note_positions: BTreeMap<usize, Position>,
}

struct KeyStore {
    payment_addresses: BTreeMap<OrderedAddress, IncomingViewingKey>,
    viewing_keys: BTreeMap<IncomingViewingKey, FullViewingKey>,
    spending_keys: BTreeMap<FullViewingKey, SpendingKey>,
    issuance_keys: BTreeMap<usize, IssuanceAuthorizingKey>,
}

impl KeyStore {
    pub fn empty() -> Self {
        KeyStore {
            payment_addresses: BTreeMap::new(),
            viewing_keys: BTreeMap::new(),
            spending_keys: BTreeMap::new(),
            issuance_keys: BTreeMap::new(),
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
        self.payment_addresses
            .insert(OrderedAddress::new(addr), ivk);
        has_fvk
    }

    pub fn add_issuance_key(&mut self, account_id: usize, iak: IssuanceAuthorizingKey) {
        self.issuance_keys.insert(account_id, iak);
    }

    pub fn spending_key_for_ivk(&self, ivk: &IncomingViewingKey) -> Option<&SpendingKey> {
        self.viewing_keys
            .get(ivk)
            .and_then(|fvk| self.spending_keys.get(fvk))
    }

    pub fn ivk_for_address(&self, addr: &Address) -> Option<&IncomingViewingKey> {
        self.payment_addresses.get(&OrderedAddress::new(*addr))
    }

    pub fn get_nullifier(&self, note: &Note) -> Option<Nullifier> {
        self.ivk_for_address(&note.recipient())
            .and_then(|ivk| self.viewing_keys.get(ivk))
            .map(|fvk| note.nullifier(fvk))
    }

    pub fn get_issuance_key(&self, account_id: usize) -> Option<&IssuanceAuthorizingKey> {
        self.issuance_keys.get(&account_id)
    }
}

pub struct Wallet {
    /// The in-memory index of keys and addresses known to the wallet.
    key_store: KeyStore,
    /// The in-memory index from txid to notes from the associated transaction that have
    /// been decrypted with the IVKs known to this wallet.
    wallet_received_notes: BTreeMap<TxId, TxNotes>,
    /// The in-memory index from txid to note positions from the associated transaction.
    /// This map should always have a subset of the keys in `wallet_received_notes`.
    wallet_note_positions: BTreeMap<TxId, NotePositions>,
    /// The in-memory index from nullifier to the outpoint of the note from which that
    /// nullifier was derived.
    nullifiers: BTreeMap<Nullifier, OutPoint>,
    /// The incremental Merkle tree used to track note commitments and witnesses for notes
    /// belonging to the wallet.
    commitment_tree: BridgeTree<MerkleHashOrchard, u32, NOTE_COMMITMENT_TREE_DEPTH>,
    /// The block height at which the last checkpoint was created, if any.
    last_checkpoint: Option<BlockHeight>,
    /// The block height and transaction index of the note most recently added to
    /// `commitment_tree`
    last_observed: Option<LastObserved>,
    /// Notes marked as mined as a consequence of their nullifiers having been observed
    /// in bundle action inputs in bundles appended to the commitment tree.  The keys of
    /// this map are the outpoints where the spent notes were created, and the values
    /// are the inpoints identifying the actions in which they are spent.
    mined_notes: BTreeMap<OutPoint, InPoint>,
    /// For each nullifier which appears more than once in transactions that this
    /// wallet has observed, the set of inpoints where those nullifiers were
    /// observed as as having been spent.
    potential_spends: BTreeMap<Nullifier, BTreeSet<InPoint>>,
}

#[derive(Debug, Clone)]
pub enum WalletError {
    OutOfOrder(LastObserved, BlockHeight, usize),
    NoteCommitmentTreeFull,
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
}

#[derive(Debug, Clone)]
pub enum SpendRetrievalError {
    DecryptedNoteNotFound(OutPoint),
    NoIvkForRecipient(Address),
    FvkNotFound(IncomingViewingKey),
    NoteNotPositioned(OutPoint),
    WitnessNotAvailableAtDepth(usize),
}

impl Wallet {
    pub fn empty() -> Self {
        Wallet {
            key_store: KeyStore::empty(),
            wallet_received_notes: BTreeMap::new(),
            wallet_note_positions: BTreeMap::new(),
            nullifiers: BTreeMap::new(),
            commitment_tree: BridgeTree::new(MAX_CHECKPOINTS),
            last_checkpoint: None,
            last_observed: None,
            mined_notes: BTreeMap::new(),
            potential_spends: BTreeMap::new(),
        }
    }

    /// Reset the state of the wallet to be suitable for rescan from the NU5 activation
    /// height.  This removes all witness and spentness information from the wallet. The
    /// keystore is unmodified and decrypted note, nullifier, and conflict data are left
    /// in place with the expectation that they will be overwritten and/or updated in
    /// the rescan process.
    pub fn reset(&mut self) {
        self.wallet_note_positions.clear();
        self.commitment_tree = BridgeTree::new(MAX_CHECKPOINTS);
        self.last_checkpoint = None;
        self.last_observed = None;
        self.mined_notes = BTreeMap::new();
    }

    /// Checkpoints the note commitment tree. This returns `false` and leaves the note
    /// commitment tree unmodified if the block height does not immediately succeed
    /// the last checkpointed block height (unless the note commitment tree is empty,
    /// in which case it unconditionally succeeds). This must be called exactly once
    /// per block.
    pub fn checkpoint(&mut self, block_height: BlockHeight) -> bool {
        // checkpoints must be in order of sequential block height and every
        // block must be checkpointed
        if let Some(last_height) = self.last_checkpoint {
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

        self.commitment_tree.checkpoint(block_height.into());
        self.last_checkpoint = Some(block_height);
        true
    }

    /// Returns the last checkpoint if any. If no checkpoint exists, the wallet has not
    /// yet observed any blocks.
    pub fn last_checkpoint(&self) -> Option<BlockHeight> {
        self.last_checkpoint
    }

    /// Rewinds the note commitment tree to the given height, removes notes and spentness
    /// information for transactions mined in the removed blocks, and returns the height to which
    /// the tree has been rewound if successful. Returns  `RewindError` if not enough checkpoints
    /// exist to execute the full rewind requested and the wallet has witness information that
    /// would be invalidated by the rewind. If the requested height is greater than or equal to the
    /// height of the latest checkpoint, this returns a successful result containing the height of
    /// the last checkpoint.
    ///
    /// In the case that no checkpoints exist but the note commitment tree also records no witness
    /// information, we allow the wallet to continue to rewind, under the assumption that the state
    /// of the note commitment tree will be overwritten prior to the next append.
    pub fn rewind(&mut self, to_height: BlockHeight) -> Result<BlockHeight, RewindError> {
        if let Some(checkpoint_height) = self.last_checkpoint {
            if to_height >= checkpoint_height {
                info!("Last checkpoint is before the rewind height, nothing to do.");
                return Ok(checkpoint_height);
            }

            info!("Rewinding note commitment tree");
            let blocks_to_rewind = <u32>::from(checkpoint_height) - <u32>::from(to_height);
            let checkpoint_count = self.commitment_tree.checkpoints().len();
            for _ in 0..blocks_to_rewind {
                // If the rewind fails, we have no more checkpoints. This is fine in the
                // case that we have a recently-initialized tree, so long as we have no
                // witnessed indices. In the case that we have any witnessed notes, we
                // have hit the maximum rewind limit, and this is an error.
                if !self.commitment_tree.rewind() {
                    assert!(self.commitment_tree.checkpoints().is_empty());
                    if !self.commitment_tree.marked_indices().is_empty() {
                        return Err(RewindError::InsufficientCheckpoints(checkpoint_count));
                    }
                }
            }

            // retain notes that correspond to transactions that are not "un-mined" after
            // the rewind
            let to_retain: BTreeSet<_> = self
                .wallet_note_positions
                .iter()
                .filter_map(|(txid, n)| {
                    if n.tx_height <= to_height {
                        Some(*txid)
                    } else {
                        None
                    }
                })
                .collect();
            info!("Retaining notes in transactions {:?}", to_retain);

            self.mined_notes.retain(|_, v| to_retain.contains(&v.txid));

            // nullifier and received note data are retained, because these values are stable
            // once we've observed a note for the first time. The block height at which we
            // observed the note is removed along with the note positions, because the
            // transaction will no longer have been observed as having been mined.
            self.wallet_note_positions
                .retain(|txid, _| to_retain.contains(txid));

            // reset our last observed height to ensure that notes added in the future are
            // from a new block
            self.last_observed = Some(LastObserved {
                block_height: to_height,
                block_tx_idx: None,
            });

            self.last_checkpoint = if checkpoint_count > blocks_to_rewind as usize {
                Some(to_height)
            } else {
                // checkpoint_count <= blocks_to_rewind
                None
            };

            Ok(to_height)
        } else if self.commitment_tree.marked_indices().is_empty() {
            info!("No witnessed notes in tree, allowing rewind without checkpoints");

            // If we have no witnessed notes, it's okay to keep "rewinding" even though
            // we have no checkpoints. We then allow last_observed to assume the height
            // to which we have reset the tree state.
            self.last_observed = Some(LastObserved {
                block_height: to_height,
                block_tx_idx: None,
            });

            Ok(to_height)
        } else {
            Err(RewindError::InsufficientCheckpoints(0))
        }
    }

    /// Add note data from all V5 transactions of the block to the wallet.
    /// Versions other than V5 are ignored.
    pub fn add_notes_from_block(&mut self, block: &Block) {
        block.transactions.iter().for_each( |tx| if tx.version() == 5 {
            self.add_notes_from_tx(tx)
        })
    }

    /// Add note data to the wallet, and return a a data structure that describes
    /// the actions that are involved with this wallet.
    pub fn add_notes_from_tx(&mut self, tx: Transaction::V5) {
        let mut issued_notes_offset = 0;

        // TODO Add transparent notes

        // Add note from Orchard bundle
        if let Some(bundle) = tx.orchard_shielded_data() {
            issued_notes_offset = bundle.actions().len();
            self.add_notes_from_orchard_bundle(tx.id, bundle);
        };

        // Add notes from Issue bundle
        if let Some(issue_bundle) = tx.issue_bundle() {
            self.add_notes_from_issue_bundle(tx.id, issue_bundle, issued_notes_offset);
        };
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

        for (action_idx, ivk, note, recipient, memo) in bundle.decrypt_outputs_with_keys(&keys) {
            assert!(self.add_decrypted_note(txid, action_idx, ivk.clone(), note, recipient, memo));
        }
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
                assert!(self.add_decrypted_note(
                    txid,
                    note_index,
                    ivk.clone(),
                    *note,
                    note.recipient(),
                    [0; 512]
                ));
            }
        }
    }

    /// Restore note and potential spend data from a bundle using the provided
    /// metadata.
    ///
    /// - `txid`: The ID for the transaction from which the provided bundle was
    ///   extracted.
    /// - `bundle`: the bundle to decrypt notes from
    /// - `hints`: a map from action index to the incoming viewing key that decrypts
    ///   that action. If the IVK does not decrypt the action, or if it is not
    ///   associated with a FVK in this wallet, `load_bundle` will return an error.
    /// - `potential_spend_idxs`: a list of action indices that were previously
    ///   detected as spending our notes. If an index is out of range, `load_bundle`
    ///   will return an error.
    pub fn load_bundle(
        &mut self,
        txid: &TxId,
        bundle: &Bundle<Authorized, Amount>,
        hints: BTreeMap<usize, &IncomingViewingKey>,
        potential_spend_idxs: &[u32],
    ) -> Result<(), BundleLoadError> {
        for action_idx in potential_spend_idxs {
            let action_idx: usize = (*action_idx).try_into().unwrap();
            if action_idx < bundle.actions().len() {
                self.add_potential_spend(
                    bundle.actions()[action_idx].nullifier(),
                    InPoint {
                        txid: *txid,
                        action_idx,
                    },
                );
            } else {
                return Err(BundleLoadError::InvalidActionIndex(action_idx));
            }
        }

        for (action_idx, ivk) in hints.into_iter() {
            if let Some((note, recipient, memo)) = bundle.decrypt_output_with_key(action_idx, ivk) {
                if !self.add_decrypted_note(txid, action_idx, ivk.clone(), note, recipient, memo) {
                    return Err(BundleLoadError::FvkNotFound(ivk.clone()));
                }
            } else {
                return Err(BundleLoadError::ActionDecryptionFailed(action_idx));
            }
        }

        Ok(())
    }

    // Common functionality for add_notes_from_bundle and load_bundle
    #[allow(clippy::too_many_arguments)]
    fn add_decrypted_note(
        &mut self,
        txid: &TxId,
        action_idx: usize,
        ivk: IncomingViewingKey,
        note: Note,
        recipient: Address,
        memo: [u8; 512],
    ) -> bool {
        // Generate the nullifier for the received note and add it to the nullifiers map so
        // that we can detect when the note is later spent.
        if let Some(fvk) = self.key_store.viewing_keys.get(&ivk) {
            info!("Adding decrypted note to the wallet");
            let outpoint = OutPoint {
                txid: *txid,
                action_idx,
            };

            // Generate the nullifier for the received note and add it to the nullifiers map so
            // that we can detect when the note is later spent.
            let nf = note.nullifier(fvk);
            self.nullifiers.insert(nf, outpoint);

            // add the decrypted note data to the wallet
            let note_data = DecryptedNote { note, memo };
            self.wallet_received_notes
                .entry(*txid)
                .or_insert_with(|| TxNotes {
                    decrypted_notes: BTreeMap::new(),
                })
                .decrypted_notes
                .insert(action_idx, note_data);

            // add the association between the address and the IVK used
            // to decrypt the note
            self.key_store.add_raw_address(recipient, ivk.clone());

            true
        } else {
            info!("Can't add decrypted note to the wallet, missing FVK");
            false
        }
    }

    /// For each Orchard action in the provided bundle, if the wallet
    /// is tracking a note corresponding to the action's revealed nullifer,
    /// mark that note as potentially spent.
    pub fn add_potential_spends(
        &mut self,
        txid: &TxId,
        bundle: &Bundle<Authorized, Amount>,
    ) -> Vec<usize> {
        // Check for spends of our notes by matching against the nullifiers
        // we're tracking, and when we detect one, associate the current
        // txid and action as spending the note.
        let mut spend_action_idxs = vec![];
        for (action_idx, action) in bundle.actions().iter().enumerate() {
            let nf = action.nullifier();
            // If a nullifier corresponds to one of our notes, add its inpoint as a
            // potential spend (the transaction may not end up being mined).
            if self.nullifiers.contains_key(nf) {
                self.add_potential_spend(
                    nf,
                    InPoint {
                        txid: *txid,
                        action_idx,
                    },
                );
                spend_action_idxs.push(action_idx);
            }
        }
        spend_action_idxs
    }

    fn add_potential_spend(&mut self, nf: &Nullifier, inpoint: InPoint) {
        info!(
            "Adding potential spend of nullifier {:?} in {:?}",
            nf,
            inpoint
        );
        self.potential_spends
            .entry(*nf)
            .or_insert_with(BTreeSet::new)
            .insert(inpoint);
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
    pub fn append_bundle_commitments(
        &mut self,
        block_height: BlockHeight,
        block_tx_idx: usize,
        txid: &TxId,
        bundle_opt: Option<&Bundle<Authorized, Amount>>,
        issue_bundle_opt: Option<&IssueBundle<Signed>>,
    ) -> Result<(), WalletError> {
        // Check that the wallet is in the correct state to update the note commitment tree with
        // new outputs.
        if let Some(last) = &self.last_observed {
            if !(
                // we are observing a subsequent transaction in the same block
                (block_height == last.block_height && last.block_tx_idx.map_or(false, |idx| idx < block_tx_idx))
                    // or we are observing a new block
                    || block_height > last.block_height
            ) {
                return Err(WalletError::OutOfOrder(
                    last.clone(),
                    block_height,
                    block_tx_idx,
                ));
            }
        }

        self.last_observed = Some(LastObserved {
            block_height,
            block_tx_idx: Some(block_tx_idx),
        });

        // update the block height recorded for the transaction
        let my_notes_for_tx = self.wallet_received_notes.get(txid);
        if my_notes_for_tx.is_some() {
            info!("Tx is ours, marking as mined");
            assert!(self
                .wallet_note_positions
                .insert(
                    *txid,
                    NotePositions {
                        tx_height: block_height,
                        note_positions: BTreeMap::default(),
                    },
                )
                .is_none());
        }

        // Process note commitments
        let mut note_commitments: Vec<ExtractedNoteCommitment> = if let Some(bundle) = bundle_opt {
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
            if my_notes_for_tx
                .as_ref()
                .and_then(|n| n.decrypted_notes.get(&note_index))
                .is_some()
            {
                info!("Witnessing Orchard note ({}, {})", txid, note_index);
                let pos = self.commitment_tree.mark().expect("tree is not empty");
                assert!(self
                    .wallet_note_positions
                    .get_mut(txid)
                    .expect("We created this above")
                    .note_positions
                    .insert(note_index, pos)
                    .is_none());
            }
        }

        // For nullifiers that are ours that we detect as spent by this action,
        // we will record that input as being mined.
        if let Some(bundle) = bundle_opt {
            for (action_idx, action) in bundle.actions().iter().enumerate() {
                if let Some(outpoint) = self.nullifiers.get(action.nullifier()) {
                    assert!(self
                        .mined_notes
                        .insert(
                            *outpoint,
                            InPoint {
                                txid: *txid,
                                action_idx,
                            },
                        )
                        .is_none());
                }
            }
        }

        Ok(())
    }

    pub fn get_filtered_notes(
        &self,
        ivk: Option<&IncomingViewingKey>,
        ignore_mined: bool,
        require_spending_key: bool,
        asset: Option<AssetBase>,
    ) -> Vec<(OutPoint, DecryptedNote)> {
        info!("Filtering notes");
        self.wallet_received_notes
            .iter()
            .flat_map(|(txid, tx_notes)| {
                tx_notes
                    .decrypted_notes
                    .iter()
                    .filter_map(move |(idx, dnote)| {
                        if asset.is_some() && asset.unwrap() != dnote.note.asset() {
                            return None;
                        }

                        let outpoint = OutPoint {
                            txid: *txid,
                            action_idx: *idx,
                        };

                        self.key_store
                            .ivk_for_address(&dnote.note.recipient())
                            // if `ivk` is `None`, return all notes that match the other conditions
                            .filter(|dnote_ivk| ivk.map_or(true, |ivk| &ivk == dnote_ivk))
                            .and_then(|dnote_ivk| {
                                if (ignore_mined && self.mined_notes.contains_key(&outpoint))
                                    || (require_spending_key
                                    && self.key_store.spending_key_for_ivk(dnote_ivk).is_none())
                                {
                                    None
                                } else {
                                    info!("Selected note at {:?}", outpoint);
                                    Some((outpoint, (*dnote).clone()))
                                }
                            })
                    })
            })
            .collect()
    }

    /// Fetches the information necessary to spend the note at the given `OutPoint`,
    /// relative to the specified root of the Orchard note commitment tree.
    ///
    /// Returns `None` if the `OutPoint` is not known to the wallet, or the Orchard bundle
    /// containing the note had not been passed to `Wallet::append_bundle_commitments` at
    /// the specified checkpoint depth.
    pub fn get_spend_info(
        &self,
        outpoint: OutPoint,
        checkpoint_depth: usize,
    ) -> Result<OrchardSpendInfo, SpendRetrievalError> {
        info!("Searching for {:?}", outpoint);
        let dnote = self
            .wallet_received_notes
            .get(&outpoint.txid)
            .and_then(|tx_notes| tx_notes.decrypted_notes.get(&outpoint.action_idx))
            .ok_or(SpendRetrievalError::DecryptedNoteNotFound(outpoint))?;
        info!("Found decrypted note for outpoint: {:?}", dnote);

        let fvk = self
            .key_store
            .ivk_for_address(&dnote.note.recipient())
            .ok_or_else(|| SpendRetrievalError::NoIvkForRecipient(dnote.note.recipient()))
            .and_then(|ivk| {
                self.key_store
                    .viewing_keys
                    .get(ivk)
                    .ok_or_else(|| SpendRetrievalError::FvkNotFound(ivk.clone()))
            })?;
        info!("Found FVK for note");

        let position = self
            .wallet_note_positions
            .get(&outpoint.txid)
            .and_then(|tx_notes| tx_notes.note_positions.get(&outpoint.action_idx))
            .ok_or(SpendRetrievalError::NoteNotPositioned(outpoint))?;
        info!("Found position for note: {:?}", position);

        assert_eq!(
            self.commitment_tree
                .get_marked_leaf(*position)
                .expect("This note has been marked as one of ours."),
            &MerkleHashOrchard::from_cmx(&dnote.note.commitment().into()),
        );

        let path = self
            .commitment_tree
            .witness(*position, checkpoint_depth)
            .map_err(|_| SpendRetrievalError::WitnessNotAvailableAtDepth(checkpoint_depth))?;

        Ok(OrchardSpendInfo::from_parts(
            fvk.clone(),
            dnote.note,
            MerklePath::from_parts(
                u64::from(*position).try_into().unwrap(),
                path.try_into().unwrap(),
            ),
        ))
    }
}