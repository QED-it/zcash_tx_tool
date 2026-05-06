use crate::components::block_data;
use crate::components::miner::MinerKey;
use crate::components::rpc_client::{BlockProposal, BlockTemplate, RpcClient};
use crate::components::wallet::Wallet;
use diesel::SqliteConnection;
use crate::components::block_commitment::{
    block_commitment_from_parts, AuthDataRoot, TxMerkleRoot, AUTH_COMMITMENT_PLACEHOLDER,
};
use crate::prelude::info;
use orchard::issuance::{IssueInfo, auth::IssueValidatingKey};
use orchard::note::{AssetId, AssetBase};
use orchard::value::NoteValue;
use orchard::Address;
use orchard::keys::Scope;
use rand::rngs::OsRng;
use std::error::Error;
use std::convert::TryFrom;
use std::ops::Add;
use orchard::keys::SpendAuthorizingKey;
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

pub fn mine(
    conn: &mut SqliteConnection,
    wallet: &mut Wallet,
    rpc_client: &mut dyn RpcClient,
    txs: Vec<Transaction>,
) -> Result<(), Box<dyn Error>> {
    let activate = wallet.last_block_height().map(u32::from).unwrap_or(0) == 0;
    mine_block(rpc_client, txs, activate)?;
    sync(conn, wallet, rpc_client);
    Ok(())
}

pub fn mine_block(
    rpc_client: &mut dyn RpcClient,
    txs: Vec<Transaction>,
    activate: bool,
) -> Result<(u32, TxId), Box<dyn Error>> {
    let block_template = rpc_client.get_block_template()?;
    let block_height = block_template.height;

    let block_proposal = template_into_proposal(block_template, txs, activate);
    let coinbase_txid = block_proposal.transactions.first().unwrap().txid();

    rpc_client.submit_block(block_proposal)?;

    Ok((block_height, coinbase_txid))
}

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

pub fn create_shield_coinbase_transaction(
    recipient: Address,
    coinbase_txid: TxId,
    rpc_client: &dyn RpcClient,
    wallet: &mut Wallet,
    miner_key: &MinerKey,
) -> Transaction {
    info!("Shielding coinbase output from tx {}", coinbase_txid);
    let target_height = rpc_client
        .get_target_height()
        .expect("failed to get target height");
    let mut tx = create_tx(target_height, wallet);

    let coinbase_amount = Zatoshis::from_u64(COINBASE_VALUE).unwrap();
    let coinbase_recipient = miner_key.address();
    let pk = miner_key.secret_key().public_key(&Secp256k1::new());

    tx.add_transparent_input(
        pk,
        OutPoint::new(coinbase_txid.into(), 0),
        TxOut::new(coinbase_amount, coinbase_recipient.script().into()),
    )
    .unwrap();
    tx.add_orchard_output::<FeeError>(
        Some(wallet.orchard_ovk()),
        recipient,
        COINBASE_VALUE,
        AssetBase::zatoshi(),
        MemoBytes::empty(),
    )
    .unwrap();

    build_tx(tx, &miner_key.signing_set(), &[], None)
}

pub fn sync(conn: &mut SqliteConnection, wallet: &mut Wallet, rpc: &mut dyn RpcClient) {
    let current_height = match wallet.last_block_height() {
        None => 0,
        Some(height) => height.add(1).into(),
    };
    sync_from_height(conn, current_height, wallet, rpc);
}

/// Sync the user with the node from the given height.
///
/// On each run the stored chain head is compared to the live chain. If it
/// matches and the persisted wallet state is consistent with `block_data`, we
/// resume from where we left off. Any inconsistency (wallet state ahead of /
/// out of sync with `block_data`, or a chain reorg detected at the stored head)
/// is treated as a hard failure: we wipe everything and resync from
/// `from_height`. We do not attempt per-block rollback or partial rewinds.
pub fn sync_from_height(
    conn: &mut SqliteConnection,
    from_height: u32,
    wallet: &mut Wallet,
    rpc: &mut dyn RpcClient,
) {
    info!("Starting sync from height {}", from_height);

    let start_height = match block_data::last_height(conn) {
        Some(head) if head_matches_chain(conn, head, rpc) => match wallet.last_block_height() {
            Some(wallet_head) if wallet_head_matches_block_data(conn, wallet) => {
                let resume = u32::from(wallet_head) + 1;
                info!("Stored head {} valid, resuming from {}", head, resume);
                // Honour the resume position; `from_height` is only an
                // activation hint for cold syncs and would create a gap if
                // it ever exceeded `wallet_head + 1`.
                resume
            }
            Some(wallet_head) => {
                info!(
                    "Wallet state at height {} does not match stored block data; \
                     clearing all persisted data and resyncing from block 0",
                    u32::from(wallet_head),
                );
                wallet.reset(conn);
                0
            }
            None => {
                info!(
                    "Stored block data valid, rebuilding wallet state from {}",
                    from_height
                );
                from_height
            }
        },
        Some(head) => {
            info!(
                "Chain reorganization detected at stored head {}; clearing all \
                         persisted data and resyncing from block 0",
                head,
            );
            wallet.reset(conn);
            0
        }
        None => {
            if wallet.last_block_height().is_some() {
                info!(
                    "Wallet state exists but block data is empty; \
                     clearing all persisted state and resyncing from block 0"
                );
                wallet.reset(conn);
                0
            } else {
                info!("No block data found, starting from {}", from_height);
                from_height
            }
        }
    };

    // Catch up in passes: read the chain tip, drain start_height..=tip, then
    // re-read the tip in case the chain advanced during the pass. Loop exits
    // when no new blocks appeared in the last pass.
    let mut next_height = start_height;
    loop {
        let target = rpc
            .get_target_height()
            .expect("failed to get target height");
        let chain_tip = u32::from(target).saturating_sub(1);

        if next_height > chain_tip {
            info!("Synced up to height {}", chain_tip);
            return;
        }

        for h in next_height..=chain_tip {
            let block = rpc
                .get_block(h)
                .unwrap_or_else(|e| panic!("RPC error fetching block {}: {}", h, e));
            info!(
                "Adding transactions from block {} at height {}",
                block.hash, block.height
            );
            let transactions = block
                .tx_ids
                .iter()
                .map(|tx_id| {
                    rpc.get_transaction(tx_id).unwrap_or_else(|e| {
                        panic!("RPC error fetching tx {} in block {}: {}", tx_id, h, e)
                    })
                })
                .collect();
            wallet
                .process_block(conn, block.height, block.hash, transactions)
                .expect("process_block");
        }
        next_height = chain_tip + 1;
    }
}

fn head_matches_chain(conn: &mut SqliteConnection, height: u32, rpc: &mut dyn RpcClient) -> bool {
    let Some(stored_hash) = block_data::get_hash(conn, height) else {
        return false;
    };
    match rpc.get_block(height) {
        Ok(block) => {
            let chain_hash = hex::encode(block.hash.0);
            if chain_hash == stored_hash {
                return true;
            }
            info!(
                "Block hash mismatch at height {}: stored {} vs chain {}",
                height, stored_hash, chain_hash
            );
            false
        }
        Err(_) => {
            info!("Block at height {} not found on chain", height);
            false
        }
    }
}

fn wallet_head_matches_block_data(conn: &mut SqliteConnection, wallet: &Wallet) -> bool {
    let (Some(height), Some(hash)) = (wallet.last_block_height(), wallet.last_block_hash()) else {
        return false;
    };
    block_data::get_hash(conn, u32::from(height))
        .map(|stored_hash| stored_hash == hex::encode(hash.0))
        .unwrap_or(false)
}

pub fn create_transfer_transaction(
    conn: &mut SqliteConnection,
    sender: Address,
    recipient: Address,
    amount: u64,
    asset: AssetBase,
    rpc_client: &dyn RpcClient,
    wallet: &mut Wallet,
) -> Transaction {
    info!("Transfer {} units", amount);

    let ovk = wallet.orchard_ovk();

    let inputs = wallet.select_spendable_notes(conn, sender, amount, asset);
    let total_inputs_amount = inputs
        .iter()
        .fold(0, |acc, input| acc + input.note.value().inner());

    info!(
        "Total inputs amount: {}, amount to transfer: {}",
        total_inputs_amount, amount
    );

    let target_height = rpc_client
        .get_target_height()
        .expect("failed to get target height");
    let mut tx = create_tx(target_height, wallet);

    let orchard_keys: Vec<SpendAuthorizingKey> = inputs
        .into_iter()
        .map(|input| {
            tx.add_orchard_spend::<FeeError>((&input.sk).into(), input.note, input.merkle_path)
                .unwrap();
            SpendAuthorizingKey::from(&input.sk)
        })
        .collect();

    tx.add_orchard_output::<FeeError>(
        Some(ovk.clone()),
        recipient,
        amount,
        asset,
        MemoBytes::empty(),
    )
    .unwrap();

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
        &TransparentSigningSet::new(),
        orchard_keys.as_slice(),
        None,
    )
}

pub fn create_burn_transaction(
    conn: &mut SqliteConnection,
    arsonist: Address,
    amount: u64,
    asset: AssetBase,
    rpc_client: &dyn RpcClient,
    wallet: &mut Wallet,
) -> Transaction {
    info!("Burn {} units", amount);

    let inputs = wallet.select_spendable_notes(conn, arsonist, amount, asset);
    let total_inputs_amount = inputs
        .iter()
        .fold(0, |acc, input| acc + input.note.value().inner());

    info!(
        "Total inputs amount: {}, amount to burn: {}",
        total_inputs_amount, amount
    );

    let target_height = rpc_client
        .get_target_height()
        .expect("failed to get target height");
    let mut tx = create_tx(target_height, wallet);

    let orchard_keys: Vec<SpendAuthorizingKey> = inputs
        .into_iter()
        .map(|input| {
            tx.add_orchard_spend::<FeeError>((&input.sk).into(), input.note, input.merkle_path)
                .unwrap();
            SpendAuthorizingKey::from(&input.sk)
        })
        .collect();

    tx.add_burn::<FeeError>(amount, asset).unwrap();

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
        &TransparentSigningSet::new(),
        orchard_keys.as_slice(),
        None,
    )
}

pub fn create_issue_transaction(
    recipient: Address,
    amount: u64,
    asset_desc_hash: [u8; 32],
    first_issuance: bool,
    rpc_client: &dyn RpcClient,
    wallet: &mut Wallet,
) -> (Transaction, AssetBase) {
    info!("Issue {} asset", amount);
    let target_height = rpc_client
        .get_target_height()
        .expect("failed to get target height");
    let dummy_recipient = wallet.address_for_account(0, Scope::External);
    let mut tx = create_tx(target_height, wallet);
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

    let asset = AssetBase::custom(&AssetId::new_v0(
        &IssueValidatingKey::from(&wallet.issuance_key()),
        &asset_desc_hash,
    ));

    // New librustzcash requires an OrchardZSA bundle with at least one action so we can
    // derive rho from the first nullifier.
    // IMPORTANT: this dummy action must be in zatoshi; Orchard can't pad output-only custom assets
    // (it needs a real spend of that asset), otherwise it panics with `NoSplitNoteAvailable`.
    tx.add_orchard_output::<FeeError>(
        Some(wallet.orchard_ovk()),
        dummy_recipient,
        0,
        AssetBase::zatoshi(),
        MemoBytes::empty(),
    )
    .unwrap();

    (
        build_tx(
            tx,
            &TransparentSigningSet::new(),
            &[],
            first_issuance.then_some(asset),
        ),
        asset,
    )
}

pub fn create_finalization_transaction(
    asset_desc_hash: [u8; 32],
    rpc_client: &dyn RpcClient,
    wallet: &mut Wallet,
) -> Transaction {
    info!("Finalize asset");
    let target_height = rpc_client
        .get_target_height()
        .expect("failed to get target height");
    let dummy_recipient = wallet.address_for_account(0, Scope::External);
    let mut tx = create_tx(target_height, wallet);
    tx.init_issuance_bundle::<FeeError>(wallet.issuance_key(), asset_desc_hash, None, false)
        .unwrap();
    tx.finalize_asset::<FeeError>(&asset_desc_hash).unwrap();

    let asset = AssetBase::custom(&AssetId::new_v0(
        &IssueValidatingKey::from(&wallet.issuance_key()),
        &asset_desc_hash,
    ));

    // Same reason as in create_issue_transaction: force at least one Orchard action.
    // Use zatoshi to avoid Orchard's custom-asset padding requirement.
    tx.add_orchard_output::<FeeError>(
        Some(wallet.orchard_ovk()),
        dummy_recipient,
        0,
        AssetBase::zatoshi(),
        MemoBytes::empty(),
    )
    .unwrap();

    build_tx(tx, &TransparentSigningSet::new(), &[], Some(asset))
}

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
            .collect::<TxMerkleRoot>()
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

fn create_tx(target_height: BlockHeight, wallet: &Wallet) -> Builder<'_, RegtestNetwork, ()> {
    let build_config = BuildConfig::TxV6 {
        sapling_anchor: None,
        orchard_anchor: wallet.orchard_anchor(),
    };
    Builder::new(REGTEST_NETWORK, target_height, build_config)
}

fn build_tx(
    builder: Builder<'_, RegtestNetwork, ()>,
    tss: &TransparentSigningSet,
    orchard_saks: &[SpendAuthorizingKey],
    new_asset: Option<AssetBase>,
) -> Transaction {
    // FIXME: the last arg of `non_standard` (creation_cost) is set to 0, use proper value instead
    let fee_rule = &FeeRule::non_standard(Zatoshis::from_u64(0).unwrap(), 20, 150, 34, 0).unwrap();
    let prover = LocalTxProver::with_default_location();
    match prover {
        None => {
            panic!("Zcash parameters not found. Please run `zcutil/fetch-params.sh`")
        }
        Some(prover) => {
            let tx = builder
                .build(
                    tss,
                    &[],
                    orchard_saks,
                    OsRng,
                    &prover,
                    &prover,
                    fee_rule,
                    |asset_base| new_asset.as_ref() == Some(asset_base),
                )
                .unwrap()
                .into_transaction();
            info!("Build tx: {}", tx.txid());
            tx
        }
    }
}
