use crate::components::block_data::BlockData;
use crate::components::rpc_client::{BlockProposal, BlockTemplate, RpcClient};
use crate::components::user::User;
use crate::components::zebra_merkle::{
    block_commitment_from_parts, AuthDataRoot, Root, AUTH_COMMITMENT_PLACEHOLDER,
};
use crate::prelude::{debug, info};
use orchard::issuance::IssueInfo;
use orchard::note::AssetBase;
use orchard::value::NoteValue;
use orchard::Address;
use rand::rngs::OsRng;
use std::error::Error;
use std::convert::TryFrom;
use std::ops::Add;
use orchard::keys::{IssuanceValidatingKey, SpendAuthorizingKey};
use secp256k1::Secp256k1;
use zcash_primitives::block::{BlockHash, BlockHeader, BlockHeaderData};
use zcash_protocol::consensus::{BlockHeight, BranchId, RegtestNetwork, REGTEST_NETWORK};
use zcash_protocol::memo::MemoBytes;
use zcash_primitives::transaction::builder::{BuildConfig, Builder};
use zcash_primitives::transaction::fees::zip317::{FeeError, FeeRule};
use zcash_primitives::transaction::{Transaction, TxId};
use zcash_proofs::prover::LocalTxProver;
use zcash_protocol::value::Zatoshis;
use zcash_transparent::builder::TransparentSigningSet;
use zcash_transparent::bundle::{OutPoint, TxOut};

const COINBASE_VALUE: u64 = 625_000_000;

/// Mine a block with the given transactions and sync the user
pub fn mine(
    wallet: &mut User,
    rpc_client: &mut dyn RpcClient,
    txs: Vec<Transaction>,
) -> Result<(), Box<dyn Error>> {
    let activate = wallet.last_block_height().is_none();
    let (_, _) = mine_block(rpc_client, txs, activate)?;
    // Only sync if mining succeeded
    sync(wallet, rpc_client);
    Ok(())
}

/// Mine a block with the given transactions and return the block height and coinbase txid
/// Retries up to MAX_MINE_RETRIES times if block is rejected (e.g., due to race condition on shared testnet)
pub fn mine_block(
    rpc_client: &mut dyn RpcClient,
    txs: Vec<Transaction>,
    activate: bool,
) -> Result<(u32, TxId), Box<dyn Error>> {
    // Shared testnet is noisy; give ourselves more chances with a longer backoff.
    const MAX_MINE_RETRIES: u32 = 10;
    const RETRY_DELAY_MS: u64 = 1_000;
    mine_block_with_retries(rpc_client, txs, activate, MAX_MINE_RETRIES, RETRY_DELAY_MS)
}

/// Mine a block but control retry policy.
/// Use `max_retries = 1` for "expected rejection" tests to avoid wasting time.
pub fn mine_block_with_retries(
    rpc_client: &mut dyn RpcClient,
    txs: Vec<Transaction>,
    activate: bool,
    max_retries: u32,
    retry_delay_ms: u64,
) -> Result<(u32, TxId), Box<dyn Error>> {
    // Serialize transactions once for potential retries
    let tx_bytes: Vec<Vec<u8>> = txs
        .iter()
        .map(|tx| {
            let mut bytes = vec![];
            tx.write(&mut bytes).unwrap();
            bytes
        })
        .collect();

    for attempt in 1..=max_retries {
        let block_template = rpc_client.get_block_template()?;
        let block_height = block_template.height;

        // Re-parse transactions from bytes for each attempt
        let txs_for_attempt: Vec<Transaction> = tx_bytes
            .iter()
            .map(|bytes| Transaction::read(&bytes[..], BranchId::Nu6).unwrap())
            .collect();

        let block_proposal = template_into_proposal(block_template, txs_for_attempt, activate);
        let coinbase_txid = block_proposal.transactions.first().unwrap().txid();

        match rpc_client.submit_block(block_proposal) {
            Ok(_) => return Ok((block_height, coinbase_txid)),
            Err(e) if e.to_string().contains("rejected") && attempt < max_retries => {
                info!(
                    "Block rejected (attempt {}/{}), retrying after {}ms...",
                    attempt, max_retries, retry_delay_ms
                );
                std::thread::sleep(std::time::Duration::from_millis(retry_delay_ms));
                continue;
            }
            Err(e) => return Err(e),
        }
    }

    Err("Max retries exceeded for mining block".into())
}

/// Mine the given number of empty blocks and return the block height and coinbase txid of the first block
pub fn mine_empty_blocks(
    num_blocks: u32,
    rpc_client: &mut dyn RpcClient,
    activate: bool,
) -> Result<(u32, TxId), Box<dyn Error>> {
    if num_blocks == 0 {
        panic!("num_blocks must be greater than 0")
    }

    let (block_height, coinbase_txid) = mine_block(rpc_client, vec![], activate)?;

    for _ in 1..num_blocks {
        mine_block(rpc_client, vec![], false)?;
    }

    Ok((block_height, coinbase_txid))
}

/// Create a shielded coinbase transaction
pub fn create_shield_coinbase_transaction(
    recipient: Address,
    coinbase_txid: TxId,
    wallet: &mut User,
) -> Transaction {
    info!("Shielding coinbase output from tx {}", coinbase_txid);

    let mut tx = create_tx(wallet);

    let coinbase_amount = Zatoshis::from_u64(COINBASE_VALUE).unwrap();
    let miner_taddr = wallet.miner_address();

    let sk = wallet.miner_sk().public_key(&Secp256k1::new());

    tx.add_transparent_input(
        sk,
        OutPoint::new(coinbase_txid.into(), 0),
        TxOut {
            value: coinbase_amount,
            script_pubkey: miner_taddr.script(),
        },
    )
    .unwrap();
    tx.add_orchard_output::<FeeError>(
        Some(wallet.orchard_ovk()),
        recipient,
        COINBASE_VALUE,
        AssetBase::native(),
        MemoBytes::empty(),
    )
    .unwrap();

    build_tx(tx, &wallet.transparent_signing_set(), &[])
}

/// Sync the user with the node
pub fn sync(wallet: &mut User, rpc: &mut dyn RpcClient) {
    let current_height = match wallet.last_block_height() {
        None => 0,
        Some(height) => height.add(1).into(),
    };
    sync_from_height(current_height, wallet, rpc);
}

/// Sync the user with the node from the given height.
/// Uses SQLite-backed block data storage to:
/// 1. Track chain progression with block hashes
/// 2. Detect chain reorganizations by verifying block hashes match
/// 3. Handle reorgs by finding the common ancestor and rescanning from there
///
/// Note: Each wallet run must process all transactions to build its own commitment tree.
/// The storage only keeps hashes for chain validation, not transaction data.
pub fn sync_from_height(from_height: u32, wallet: &mut User, rpc: &mut dyn RpcClient) {
    info!("Starting sync from height {}", from_height);

    // Load the block data storage
    let mut block_data = BlockData::load();

    // If this is a fresh wallet, but the storage contains full tx data, rebuild the wallet
    // commitment tree locally by replaying stored blocks. This avoids re-downloading
    // all historical blocks/transactions on subsequent runs.
    #[allow(clippy::collapsible_if)]
    if wallet.last_block_height().is_none()
        && block_data.last_height().is_some()
        && block_data.has_complete_tx_data()
    {
        if let Some(replayed) = replay_stored_blocks_to_wallet(from_height, wallet, &block_data) {
            info!("Replayed stored blocks locally up to height {}", replayed);
        }
    }

    // Determine the starting height based on stored blocks and chain validation
    let start_height = determine_sync_start_height(from_height, wallet, &mut block_data, rpc);

    info!("Determined sync start height: {}", start_height);

    let mut next_height = start_height;

    loop {
        match rpc.get_block(next_height) {
            Ok(block) => {
                info!(
                    "Adding transactions from block {} at height {}",
                    block.hash, block.height
                );

                // If the storage has tx data for this height and the hash matches, use it.
                // Otherwise fetch from RPC and write it to the storage for next run.
                let (transactions, tx_hex): (Vec<Transaction>, Vec<String>) =
                    if let Some(stored) = block_data.get(next_height) {
                        let chain_hash_hex = hex::encode(block.hash.0);
                        if stored.hash == chain_hash_hex && !stored.tx_hex.is_empty() {
                            let txs = stored
                                .tx_hex
                                .iter()
                                .map(|hex_tx| {
                                    let bytes = hex::decode(hex_tx).unwrap();
                                    Transaction::read(bytes.as_slice(), BranchId::Nu6).unwrap()
                                })
                                .collect::<Vec<_>>();
                            (txs, stored.tx_hex.clone())
                        } else {
                            let txs = fetch_block_txs(&block.tx_ids, rpc);
                            let hexes = serialize_txs(&txs);
                            (txs, hexes)
                        }
                    } else {
                        let txs = fetch_block_txs(&block.tx_ids, rpc);
                        let hexes = serialize_txs(&txs);
                        (txs, hexes)
                    };

                let prev_hash = if next_height > 0 {
                    if let Some(prev_stored) = block_data.get(next_height - 1) {
                        prev_stored.hash.clone()
                    } else {
                        match rpc.get_block(next_height - 1) {
                            Ok(prev_block) => hex::encode(prev_block.hash.0),
                            Err(_) => hex::encode(block.previous_block_hash.0),
                        }
                    }
                } else {
                    hex::encode(block.previous_block_hash.0)
                };

                block_data.insert(next_height, hex::encode(block.hash.0), prev_hash, tx_hex);

                wallet
                    .add_notes_from_block(block.height, block.hash, transactions)
                    .unwrap();
                next_height += 1;
            }
            Err(err) => {
                info!(
                    "No block at height {}. Synced up to height {}",
                    next_height,
                    next_height - 1
                );
                debug!("rpc.get_block err: {:?}", err);
                // Save the block data before returning
                block_data.save();
                return;
            }
        }
    }
}

fn serialize_txs(txs: &[Transaction]) -> Vec<String> {
    txs.iter()
        .map(|tx| {
            let mut bytes = vec![];
            tx.write(&mut bytes).unwrap();
            hex::encode(bytes)
        })
        .collect()
}

fn replay_stored_blocks_to_wallet(
    from_height: u32,
    wallet: &mut User,
    block_data: &BlockData,
) -> Option<u32> {
    // Replay stored txs in ascending height order, starting from from_height.
    let mut last = None;
    for (height, stored) in block_data.blocks_iter() {
        if *height < from_height {
            continue;
        }
        if stored.tx_hex.is_empty() {
            return None;
        }

        let txs = stored
            .tx_hex
            .iter()
            .map(|hex_tx| {
                let bytes = hex::decode(hex_tx).unwrap();
                Transaction::read(bytes.as_slice(), BranchId::Nu6).unwrap()
            })
            .collect::<Vec<_>>();

        // Reconstruct BlockHash bytes from stored big-endian hex (no reversal).
        let hash_bytes = hex::decode(&stored.hash).ok()?;
        if hash_bytes.len() != 32 {
            return None;
        }
        let mut hash_arr = [0u8; 32];
        hash_arr.copy_from_slice(&hash_bytes);

        wallet
            .add_notes_from_block(BlockHeight::from_u32(*height), BlockHash(hash_arr), txs)
            .ok()?;
        last = Some(*height);
    }
    last
}

/// Determines the height from which to start syncing based on:
/// 1. The requested from_height
/// 2. The wallet's last processed block
/// 3. The stored block data with chain validation
///
/// ## Storage Behavior
///
/// This function enables two levels of persistence:
///
/// 1. **Block Data Only** (current default in tests):
///    - Tests call `wallet.reset()` which clears wallet state but keeps block data
///    - Benefit: Validates chain continuity without re-fetching block metadata
///    - Limitation: Still requires full wallet rescan (note decryption, nullifiers, etc.)
///
/// 2. **Full Persistence** (block data + wallet state):
///    - Keep wallet state between runs (don't call `reset()`)
///    - If `wallet_last_block_height > 0` AND stored data is valid:
///      → Skips the entire sync (no block fetching, no wallet scanning)
///    - Use case: Production wallets that persist between sessions
///
fn determine_sync_start_height(
    from_height: u32,
    wallet: &User,
    block_data: &mut BlockData,
    rpc: &mut dyn RpcClient,
) -> u32 {
    let wallet_last_block_height = wallet.last_block_height().map_or(0, u32::from);

    // IMPORTANT:
    // If the wallet has no local state, we must rebuild the note commitment tree from
    // `from_height`. The block data storage only tracks hashes for chain progression / reorg
    // detection; resuming from the stored tip would skip processing historical txs and
    // yield an invalid Orchard anchor, causing mined blocks to be rejected.
    //
    // To enable full persistence (skip rescan entirely), keep wallet state between runs.
    if wallet_last_block_height == 0 {
        info!(
            "Wallet has no synced blocks; rescanning from height {}",
            from_height
        );
        return from_height;
    }

    // Get the last stored block height
    let last_stored_height = block_data.last_height();

    match last_stored_height {
        Some(stored_height) => {
            let stored_block = block_data.get(stored_height).unwrap();
            info!(
                "Found stored block at height {} with hash {}",
                stored_height, stored_block.hash
            );

            // Validate the stored chain against the current blockchain
            match validate_stored_chain(stored_height, block_data, rpc) {
                ChainValidationResult::Valid => {
                    // Chain is valid, continue from after the last stored block
                    let resume_height = stored_height + 1;
                    info!(
                        "Stored data valid, resuming sync from height {}",
                        resume_height
                    );
                    resume_height.max(from_height)
                }
                ChainValidationResult::Reorg(reorg_height) => {
                    // Chain reorganization detected, need to rescan from reorg point
                    info!(
                        "Chain reorganization detected at height {}, clearing stored data from that point",
                        reorg_height
                    );
                    block_data.truncate_from(reorg_height);
                    block_data.save();
                    reorg_height.max(from_height)
                }
                ChainValidationResult::NoBlockOnChain => {
                    // Zebra node has been reset or chain data is completely different
                    // Clear all stored block data and start fresh
                    info!("No common ancestor found, clearing all stored block data and starting fresh");
                    block_data.truncate_from(1);
                    block_data.save();
                    from_height.max(wallet_last_block_height)
                }
            }
        }
        None => {
            // No stored data, use the higher of from_height or wallet's last height
            info!("No block data found, starting fresh");
            from_height.max(wallet_last_block_height)
        }
    }
}

fn fetch_block_txs(tx_ids: &[TxId], rpc: &mut dyn RpcClient) -> Vec<Transaction> {
    const MAX_TX_RETRIES: u32 = 3;
    const TX_RETRY_DELAY_MS: u64 = 1000;

    tx_ids
        .iter()
        .map(|tx_id| {
            for attempt in 1..=MAX_TX_RETRIES {
                match rpc.get_transaction(tx_id) {
                    Ok(tx) => return tx,
                    Err(e) => {
                        if attempt < MAX_TX_RETRIES {
                            info!(
                                "Failed to fetch tx {} (attempt {}/{}): {}, retrying...",
                                tx_id, attempt, MAX_TX_RETRIES, e
                            );
                            std::thread::sleep(std::time::Duration::from_millis(TX_RETRY_DELAY_MS));
                        } else {
                            panic!(
                                "Failed to fetch tx {} after {} attempts: {}",
                                tx_id, MAX_TX_RETRIES, e
                            );
                        }
                    }
                }
            }
            unreachable!()
        })
        .collect()
}

/// Result of validating the stored chain against the current blockchain
enum ChainValidationResult {
    /// The stored chain matches the current blockchain
    Valid,
    /// A reorganization was detected at the specified height
    Reorg(u32),
    /// The stored block doesn't exist on the chain
    NoBlockOnChain,
}

/// Validates that the stored chain matches the current blockchain.
/// Walks backwards from the last stored block to find where the chains diverge.
fn validate_stored_chain(
    stored_height: u32,
    block_data: &BlockData,
    rpc: &mut dyn RpcClient,
) -> ChainValidationResult {
    let stored_block = match block_data.get(stored_height) {
        Some(b) => b,
        None => return ChainValidationResult::NoBlockOnChain,
    };

    // First, check if the last stored block matches
    match rpc.get_block(stored_height) {
        Ok(chain_block) => {
            let chain_hash = hex::encode(chain_block.hash.0);
            if chain_hash == stored_block.hash {
                // Perfect match, chain is valid
                return ChainValidationResult::Valid;
            }

            // Hash mismatch - walk backwards to find where chains diverge
            info!(
                "Block hash mismatch at height {}: stored {} vs chain {}",
                stored_height, stored_block.hash, chain_hash
            );

            find_common_ancestor(stored_height, block_data, rpc)
        }
        Err(_) => {
            // Block at stored height doesn't exist on chain yet
            // Walk backwards to find the highest block that does exist and matches
            info!(
                "Block at height {} not found on chain, walking back to find valid stored point",
                stored_height
            );

            find_common_ancestor(stored_height, block_data, rpc)
        }
    }
}

/// Walk backwards through the stored data to find the fork point / common ancestor
fn find_common_ancestor(
    from_height: u32,
    block_data: &BlockData,
    rpc: &mut dyn RpcClient,
) -> ChainValidationResult {
    let mut check_height = from_height;
    while check_height > 1 {
        check_height -= 1;

        if let Some(check_stored) = block_data.get(check_height) {
            match rpc.get_block(check_height) {
                Ok(block) => {
                    let block_hash = hex::encode(block.hash.0);
                    if block_hash == check_stored.hash {
                        // Found the common ancestor, reorg starts at check_height + 1
                        info!(
                            "Found common ancestor at height {}, will resume from {}",
                            check_height,
                            check_height + 1
                        );
                        return ChainValidationResult::Reorg(check_height + 1);
                    }
                }
                Err(_) => {
                    // Block not found, continue walking back
                    continue;
                }
            }
        } else {
            // No stored block at this height, reorg starts here
            return ChainValidationResult::Reorg(check_height + 1);
        }
    }

    // Couldn't find common ancestor, treat stored data as invalid but preserve it
    info!("No common ancestor found, ignoring stored data and starting fresh");
    ChainValidationResult::NoBlockOnChain
}

/// Create a transfer transaction
pub fn create_transfer_transaction(
    sender: Address,
    recipient: Address,
    amount: u64,
    asset: AssetBase,
    wallet: &mut User,
) -> Transaction {
    info!("Transfer {} zatoshi", amount);

    let ovk = wallet.orchard_ovk();

    // Add inputs
    let inputs = wallet.select_spendable_notes(sender, amount, asset);
    let total_inputs_amount = inputs
        .iter()
        .fold(0, |acc, input| acc + input.note.value().inner());

    info!(
        "Total inputs amount: {}, amount to transfer: {}",
        total_inputs_amount, amount
    );

    let mut tx = create_tx(wallet);

    let orchard_keys: Vec<SpendAuthorizingKey> = inputs
        .into_iter()
        .map(|input| {
            tx.add_orchard_spend::<FeeError>((&input.sk).into(), input.note, input.merkle_path)
                .unwrap();
            SpendAuthorizingKey::from(&input.sk)
        })
        .collect();

    // Add main transfer output
    tx.add_orchard_output::<FeeError>(
        Some(ovk.clone()),
        recipient,
        amount,
        asset,
        MemoBytes::empty(),
    )
    .unwrap();

    // Add change output
    let change_amount = total_inputs_amount - amount;

    if change_amount != 0 {
        tx.add_orchard_output::<FeeError>(
            Some(ovk),
            sender,
            change_amount,
            asset,
            MemoBytes::empty(),
        )
        .unwrap();
    }

    build_tx(
        tx,
        &wallet.transparent_signing_set(),
        orchard_keys.as_slice(),
    )
}

/// Create a burn transaction
pub fn create_burn_transaction(
    arsonist: Address,
    amount: u64,
    asset: AssetBase,
    wallet: &mut User,
) -> Transaction {
    info!("Burn {} zatoshi", amount);

    // Add inputs
    let inputs = wallet.select_spendable_notes(arsonist, amount, asset);
    let total_inputs_amount = inputs
        .iter()
        .fold(0, |acc, input| acc + input.note.value().inner());

    info!(
        "Total inputs amount: {}, amount to burn: {}",
        total_inputs_amount, amount
    );

    let mut tx = create_tx(wallet);

    let orchard_keys: Vec<SpendAuthorizingKey> = inputs
        .into_iter()
        .map(|input| {
            tx.add_orchard_spend::<FeeError>((&input.sk).into(), input.note, input.merkle_path)
                .unwrap();
            SpendAuthorizingKey::from(&input.sk)
        })
        .collect();

    // Add main transfer output
    tx.add_burn::<FeeError>(amount, asset).unwrap();

    // Add change output if needed
    let change_amount = total_inputs_amount - amount;
    if change_amount != 0 {
        let ovk = wallet.orchard_ovk();
        tx.add_orchard_output::<FeeError>(
            Some(ovk),
            arsonist,
            change_amount,
            asset,
            MemoBytes::empty(),
        )
        .unwrap();
    }

    build_tx(
        tx,
        &wallet.transparent_signing_set(),
        orchard_keys.as_slice(),
    )
}

/// Create a transaction that issues a new asset
pub fn create_issue_transaction(
    recipient: Address,
    amount: u64,
    asset_desc_hash: [u8; 32],
    first_issuance: bool,
    wallet: &mut User,
) -> (Transaction, AssetBase) {
    info!("Issue {} asset", amount);
    let mut tx = create_tx(wallet);
    tx.init_issuance_bundle::<FeeError>(
        wallet.issuance_key(),
        asset_desc_hash,
        Some(IssueInfo {
            recipient,
            value: NoteValue::from_raw(amount),
        }),
        first_issuance,
    )
    .unwrap();
    let asset = AssetBase::derive(
        &IssuanceValidatingKey::from(&wallet.issuance_key()),
        &asset_desc_hash,
    );
    (build_tx(tx, &wallet.transparent_signing_set(), &[]), asset)
}

/// Create a transaction that issues a new asset
pub fn create_finalization_transaction(
    asset_desc_hash: [u8; 32],
    wallet: &mut User,
) -> Transaction {
    info!("Finalize asset");
    let mut tx = create_tx(wallet);
    tx.init_issuance_bundle::<FeeError>(wallet.issuance_key(), asset_desc_hash, None, false)
        .unwrap();
    tx.finalize_asset::<FeeError>(&asset_desc_hash).unwrap();
    build_tx(tx, &wallet.transparent_signing_set(), &[])
}

/// Convert a block template and a list of transactions into a block proposal
pub fn template_into_proposal(
    block_template: BlockTemplate,
    mut txs: Vec<Transaction>,
    activate: bool,
) -> BlockProposal {
    let coinbase = Transaction::read(
        hex::decode(block_template.coinbase_txn.data)
            .unwrap()
            .as_slice(),
        BranchId::Nu6,
    )
    .unwrap();

    let mut txs_with_coinbase = vec![coinbase];
    txs_with_coinbase.append(&mut txs);

    let merkle_root = if txs_with_coinbase.len() == 1 {
        // only coinbase tx is present, no need to calculate
        crate::components::rpc_client::decode_hex(block_template.default_roots.merkle_root)
    } else {
        txs_with_coinbase
            .iter()
            .map(|tx| *tx.txid().clone().as_ref())
            .collect::<Root>()
            .0
    };

    let auth_data_root = txs_with_coinbase
        .iter()
        .map(|tx| {
            if tx.version().has_orchard() || tx.version().has_orchard_zsa() {
                <[u8; 32]>::try_from(tx.auth_commitment().as_bytes()).unwrap()
            } else {
                AUTH_COMMITMENT_PLACEHOLDER
            }
        })
        .collect::<AuthDataRoot>();

    let hash_block_commitments = block_commitment_from_parts(
        crate::components::rpc_client::decode_hex(block_template.default_roots.chain_history_root),
        auth_data_root.0,
    );

    let block_header_data = BlockHeaderData {
        version: block_template.version as i32,
        prev_block: BlockHash(crate::components::rpc_client::decode_hex(
            block_template.previous_block_hash,
        )),
        merkle_root,
        final_sapling_root: if activate {
            [0; 32]
        } else {
            hash_block_commitments
        },
        time: block_template.cur_time,
        bits: u32::from_str_radix(block_template.bits.as_str(), 16).unwrap(),
        nonce: [2; 32],                 // Currently PoW is switched off in Zebra
        solution: Vec::from([0; 1344]), // Currently PoW is switched off in Zebra
    };

    let header = BlockHeader::from_data(block_header_data).unwrap();

    BlockProposal {
        header,
        transactions: txs_with_coinbase,
    }
}

fn create_tx(wallet: &User) -> Builder<'_, RegtestNetwork, ()> {
    let build_config = BuildConfig::TxV6 {
        sapling_anchor: None,
        orchard_anchor: wallet.orchard_anchor(),
    };
    Builder::new(
        REGTEST_NETWORK,
        /*user.last_block_height().unwrap()*/ BlockHeight::from_u32(1_842_420),
        build_config,
    )
}

fn build_tx(
    builder: Builder<'_, RegtestNetwork, ()>,
    tss: &TransparentSigningSet,
    orchard_saks: &[SpendAuthorizingKey],
) -> Transaction {
    let fee_rule = &FeeRule::non_standard(Zatoshis::from_u64(0).unwrap(), 20, 150, 34).unwrap();
    let prover = LocalTxProver::with_default_location();
    match prover {
        None => {
            panic!("Zcash parameters not found. Please run `zcutil/fetch-params.sh`")
        }
        Some(prover) => {
            let tx = builder
                .build(tss, &[], orchard_saks, OsRng, &prover, &prover, fee_rule)
                .unwrap()
                .into_transaction();
            info!("Build tx: {}", tx.txid());
            tx
        }
    }
}
