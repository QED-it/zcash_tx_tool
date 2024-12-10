use crate::components::rpc_client::{BlockProposal, BlockTemplate, RpcClient};
use crate::components::wallet::Wallet;
use crate::components::zebra_merkle::{
    block_commitment_from_parts, AuthDataRoot, Root, AUTH_COMMITMENT_PLACEHOLDER,
};
use crate::prelude::{debug, info};
use orchard::issuance::IssueInfo;
use orchard::note::AssetBase;
use orchard::value::NoteValue;
use orchard::Address;
use rand::rngs::OsRng;
use std::convert::TryFrom;
use std::ops::Add;
use zcash_primitives::block::{BlockHash, BlockHeader, BlockHeaderData};
use zcash_primitives::consensus::{BlockHeight, BranchId, RegtestNetwork, REGTEST_NETWORK};
use zcash_primitives::memo::MemoBytes;
use zcash_primitives::transaction::builder::{BuildConfig, Builder};
use zcash_primitives::transaction::components::amount::NonNegativeAmount;
use zcash_primitives::transaction::components::{transparent, TxOut};
use zcash_primitives::transaction::fees::zip317::{FeeError, FeeRule};
use zcash_primitives::transaction::{Transaction, TxId};
use zcash_proofs::prover::LocalTxProver;

/// Mine a block with the given transactions and sync the wallet
pub fn mine(
    wallet: &mut Wallet,
    rpc_client: &mut dyn RpcClient,
    txs: Vec<Transaction>,
    activate: bool,
) {
    let (_, _) = mine_block(rpc_client, BranchId::Nu5, txs, activate);
    sync(wallet, rpc_client);
}

/// Mine a block with the given transactions and return the block height and coinbase txid
pub fn mine_block(
    rpc_client: &mut dyn RpcClient,
    branch_id: BranchId,
    txs: Vec<Transaction>,
    activate: bool,
) -> (u32, TxId) {
    let block_template = rpc_client.get_block_template().unwrap();
    let block_height = block_template.height;

    let block_proposal = template_into_proposal(block_template, branch_id, txs, activate);
    let coinbase_txid = block_proposal.transactions.first().unwrap().txid();

    rpc_client.submit_block(block_proposal).unwrap();

    (block_height, coinbase_txid)
}

/// Mine the given number of empty blocks and return the block height and coinbase txid of the first block
pub fn mine_empty_blocks(
    num_blocks: u32,
    rpc_client: &mut dyn RpcClient,
    activate: bool,
) -> (u32, TxId) {
    if num_blocks == 0 {
        panic!("num_blocks must be greater than 0")
    }

    let (block_height, coinbase_txid) = mine_block(rpc_client, BranchId::Nu5, vec![], activate);

    for _ in 1..num_blocks {
        mine_block(rpc_client, BranchId::Nu5, vec![], false);
    }

    (block_height, coinbase_txid)
}

/// Create a shielded coinbase transaction
pub fn create_shield_coinbase_transaction(
    recipient: Address,
    coinbase_txid: TxId,
    wallet: &mut Wallet,
) -> Transaction {
    info!("Shielding coinbase output from tx {}", coinbase_txid);

    let mut tx = create_tx(wallet);

    let coinbase_value = 625_000_000;
    let coinbase_amount = NonNegativeAmount::from_u64(coinbase_value).unwrap();
    let miner_taddr = wallet.miner_address();

    let sk = wallet.miner_sk();

    tx.add_transparent_input(
        sk,
        transparent::OutPoint::new(coinbase_txid.0, 0),
        TxOut {
            value: coinbase_amount,
            script_pubkey: miner_taddr.script(),
        },
    )
    .unwrap();
    tx.add_orchard_output::<FeeError>(
        Some(wallet.orchard_ovk()),
        recipient,
        coinbase_value,
        AssetBase::native(),
        MemoBytes::empty(),
    )
    .unwrap();

    build_tx(tx)
}

/// Sync the wallet with the node
pub fn sync(wallet: &mut Wallet, rpc: &mut dyn RpcClient) {
    let current_height = match wallet.last_block_height() {
        None => 0,
        Some(height) => height.add(1).into(),
    };
    sync_from_height(current_height, wallet, rpc);
}

/// Sync the wallet with the node from the given height
pub fn sync_from_height(from_height: u32, wallet: &mut Wallet, rpc: &mut dyn RpcClient) {
    info!("Starting sync from height {}", from_height);

    let wallet_last_block_height = wallet.last_block_height().map_or(0, |h| h.into());
    let mut next_height = if from_height < wallet_last_block_height {
        wallet_last_block_height
    } else {
        from_height
    };

    loop {
        match rpc.get_block(next_height) {
            Ok(block) => {
                // if block.prev_hash != wallet.last_block_hash
                // Fork management is not implemented since block.prev_hash rpc is not yet implemented in Zebra

                info!(
                    "Adding transactions from block {} at height {}",
                    block.hash, block.height
                );
                let transactions = block
                    .tx_ids
                    .into_iter()
                    .map(|tx_id| rpc.get_transaction(&tx_id).unwrap())
                    .collect();
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
                return;
            }
        }
    }
}

/// Create a transfer transaction
pub fn create_transfer_transaction(
    sender: Address,
    recipient: Address,
    amount: u64,
    asset: AssetBase,
    wallet: &mut Wallet,
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

    inputs.into_iter().for_each(|input| {
        tx.add_orchard_spend::<FeeError>(&input.sk, input.note, input.merkle_path)
            .unwrap()
    });

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

    build_tx(tx)
}

/// Create a burn transaction
pub fn create_burn_transaction(
    arsonist: Address,
    amount: u64,
    asset: AssetBase,
    wallet: &mut Wallet,
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

    inputs.into_iter().for_each(|input| {
        tx.add_orchard_spend::<FeeError>(&input.sk, input.note, input.merkle_path)
            .unwrap()
    });

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

    build_tx(tx)
}

/// Create a transaction that issues a new asset
pub fn create_issue_transaction(
    recipient: Address,
    amount: u64,
    asset_desc: Vec<u8>,
    wallet: &mut Wallet,
) -> Transaction {
    info!("Issue {} asset", amount);
    let mut tx = create_tx(wallet);
    tx.init_issuance_bundle::<FeeError>(
        wallet.issuance_key(),
        asset_desc,
        Some(IssueInfo {
            recipient,
            value: NoteValue::from_raw(amount),
        }),
    )
    .unwrap();
    build_tx(tx)
}

/// Create a transaction that issues a new asset
pub fn create_finalization_transaction(asset_desc: Vec<u8>, wallet: &mut Wallet) -> Transaction {
    info!("Finalize asset");
    let mut tx = create_tx(wallet);
    tx.init_issuance_bundle::<FeeError>(wallet.issuance_key(), asset_desc.clone(), None)
        .unwrap();
    tx.finalize_asset::<FeeError>(asset_desc.as_slice())
        .unwrap();
    build_tx(tx)
}

/// Convert a block template and a list of transactions into a block proposal
pub fn template_into_proposal(
    block_template: BlockTemplate,
    branch_id: BranchId,
    mut txs: Vec<Transaction>,
    activate: bool,
) -> BlockProposal {
    let coinbase = Transaction::read(
        hex::decode(block_template.coinbase_txn.data)
            .unwrap()
            .as_slice(),
        branch_id,
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
            .map(|tx| tx.txid().0)
            .collect::<Root>()
            .0
    };

    let auth_data_root = txs_with_coinbase
        .iter()
        .map(|tx| {
            if tx.version().has_orchard() || tx.version().has_orchard_zsa() {
                let bytes = <[u8; 32]>::try_from(tx.auth_commitment().as_bytes()).unwrap();
                bytes
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

fn create_tx(wallet: &Wallet) -> Builder<'_, RegtestNetwork, ()> {
    let build_config = BuildConfig::Zsa {
        sapling_anchor: None,
        orchard_anchor: wallet.orchard_anchor(),
    };
    let tx = Builder::new(
        REGTEST_NETWORK,
        /*wallet.last_block_height().unwrap()*/ BlockHeight::from_u32(1_842_420),
        build_config,
    );
    tx
}

fn build_tx(builder: Builder<'_, RegtestNetwork, ()>) -> Transaction {
    let fee_rule =
        &FeeRule::non_standard(NonNegativeAmount::from_u64(0).unwrap(), 20, 150, 34).unwrap();
    let prover = LocalTxProver::with_default_location();
    match prover {
        None => {
            panic!("Zcash parameters not found. Please run `zcutil/fetch-params.sh`")
        }
        Some(prover) => {
            let tx = builder
                .build(OsRng, &prover, &prover, fee_rule)
                .unwrap()
                .into_transaction();
            info!("Build tx: {}", tx.txid());
            tx
        }
    }
}
