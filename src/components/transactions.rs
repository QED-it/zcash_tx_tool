use std::convert::TryFrom;
use orchard::Address;
use orchard::note::AssetBase;
use orchard::value::NoteValue;
use rand::rngs::OsRng;
use zcash_primitives::block::{BlockHash, BlockHeader, BlockHeaderData};
use zcash_primitives::consensus::{BlockHeight, TEST_NETWORK, TestNetwork};
use zcash_primitives::memo::MemoBytes;
use zcash_primitives::transaction::{Transaction, TxId};
use zcash_primitives::transaction::builder::Builder;
use zcash_primitives::transaction::components::{Amount, transparent, TxOut};
use zcash_primitives::transaction::fees::zip317::{FeeError, FeeRule};
use zcash_proofs::prover::LocalTxProver;
use crate::components::rpc_client::{BlockProposal, BlockTemplate, RpcClient};
use crate::components::wallet::Wallet;
use crate::components::zebra_merkle::{AUTH_COMMITMENT_PLACEHOLDER, AuthDataRoot, block_commitment_from_parts, Root};
use crate::prelude::{error, info};

/// Mine a block with the given transactions and sync the wallet
pub fn mine(wallet: &mut Wallet, rpc_client: &mut dyn RpcClient, txs: Vec<Transaction>) {
    let (block_height, _) = mine_block(rpc_client, txs);
    sync_from_height(block_height, wallet, rpc_client);
}

/// Mine a block with the given transactions and return the block height and coinbase txid
pub fn mine_block(rpc_client: &mut dyn RpcClient, txs: Vec<Transaction>) -> (u32, TxId) {
    let block_template = rpc_client.get_block_template().unwrap();
    let block_height = block_template.height;

    let block_proposal = template_into_proposal(block_template, txs);
    let coinbase_txid = block_proposal.transactions.first().unwrap().txid();

    rpc_client.submit_block(block_proposal).unwrap();

    (block_height, coinbase_txid)
}

/// Mine the given number of empty blocks and return the block height and coinbase txid of the first block
pub fn mine_empty_blocks(num_blocks: u32, rpc_client: &mut dyn RpcClient) -> (u32, TxId) {

    if num_blocks <= 0 { panic!("num_blocks must be greater than 0") }

    let (block_height, coinbase_txid) = mine_block(rpc_client, vec![]);

    for _ in 1..num_blocks {
        mine_block(rpc_client, vec![]);
    };

    (block_height, coinbase_txid)
}

/// Create a shielded coinbase transaction
pub fn create_shield_coinbase_tx(recipient: Address, coinbase_txid: TxId, wallet: &mut Wallet) -> Transaction {

    info!("Shielding coinbase output from tx {}", coinbase_txid);

    let mut tx = Builder::new(TEST_NETWORK, /*wallet.last_block_height().unwrap()*/ BlockHeight::from_u32(1_842_421), wallet.orchard_anchor());

    let coinbase_value = 500000000;
    let coinbase_amount = Amount::from_u64(coinbase_value).unwrap();
    let miner_taddr = wallet.miner_address();

    let sk = wallet.miner_sk();

    tx.add_transparent_input(sk, transparent::OutPoint::new(coinbase_txid.0, 0), TxOut { value: coinbase_amount, script_pubkey: miner_taddr.script() }).unwrap();
    tx.add_orchard_output::<FeeError>(Some(wallet.orchard_ovk()), recipient, coinbase_value, MemoBytes::empty()).unwrap();

    let fee_rule = &FeeRule::non_standard(Amount::from_u64(0).unwrap(), 20, 150, 34).unwrap();
    let prover = LocalTxProver::with_default_location().unwrap();  // TODO warn on missing params
    let (tx, _) = tx.build(&prover, fee_rule).unwrap();

    info!("TxId: {}", tx.txid());
    tx
}

/// Sync the wallet with the node
pub fn sync(wallet: &mut Wallet, rpc: &mut dyn RpcClient) {
    let current_height = wallet.last_block_height().map_or(0, |h| h.into());
    sync_from_height(current_height, wallet, rpc);
}

/// Sync the wallet with the node from the given height
pub fn sync_from_height(from_height: u32, wallet: &mut Wallet, rpc: &mut dyn RpcClient) {
    info!("Starting sync from height {}", from_height);

    let wallet_last_block_height = wallet.last_block_height().map_or(0, |h| h.into());
    let mut next_height= if from_height < wallet_last_block_height {
        wallet_last_block_height
    } else {
        from_height
    };

    loop {
        let block = match rpc.get_block(next_height) {
            Ok(block) => block,
            Err(err) => {
                info!("No block at height {}: {}", next_height, err);
                return
            }
        };

        if true /* block.prev_hash == wallet.last_block_hash */ {
            info!("Adding transactions from block {} at height {}", block.hash, block.height);
            let transactions = block.tx_ids.into_iter().map(| tx_id| rpc.get_transaction(&tx_id).unwrap()).collect();
            wallet.add_notes_from_block(block.height, block.hash, transactions).unwrap();
            next_height += 1;
        } else {
            // Fork management is not implemented
            error!("REORG: dropping block {} at height {}", wallet.last_block_hash().unwrap(), next_height);
        }
    }
}

/// Create a vanilla Orchard transfer transaction
pub fn create_transfer_tx(sender: Address, recipient: Address, amount: u64, wallet: &mut Wallet) -> Transaction {

    info!("Transfer {} zatoshi", amount);

    let ovk = wallet.orchard_ovk();

    let mut tx = create_tx(wallet);

    // Add inputs
    let inputs = wallet.select_spendable_notes(sender, amount, None);
    let total_inputs_amount = inputs.iter().fold(0, |acc, input| acc + input.note.value().inner());

    info!("Total inputs amount: {}, amount to transfer: {}", total_inputs_amount, amount);

    inputs.into_iter().for_each(|input| tx.add_orchard_spend::<FeeError>(input.sk, input.note, input.merkle_path).unwrap());

    // Add main transfer output
    tx.add_orchard_output::<FeeError>(Some(ovk.clone()), recipient, amount, MemoBytes::empty()).unwrap();

    // Add change output
    let change_amount = total_inputs_amount - amount;

    if change_amount != 0 {
        tx.add_orchard_output::<FeeError>(Some(ovk), sender, change_amount, MemoBytes::empty()).unwrap();
    }

    build_tx(tx)
}

/// Create a transfer transaction
pub fn create_transfer_zsa_tx(sender: Address, recipient: Address, amount: u64, asset: AssetBase, wallet: &mut Wallet) -> Transaction {

    info!("Transfer {} zatoshi", amount);

    let ovk = wallet.orchard_ovk();

    let mut tx = create_tx(wallet);

    // Add inputs
    let inputs = wallet.select_spendable_notes(sender, amount, Some(asset));
    let total_inputs_amount = inputs.iter().fold(0, |acc, input| acc + input.note.value().inner());

    info!("Total inputs amount: {}, amount to transfer: {}", total_inputs_amount, amount);

    inputs.into_iter().for_each(|input| tx.add_orchard_spend::<FeeError>(input.sk, input.note, input.merkle_path).unwrap());

    // Add main transfer output
    tx.add_orchard_zsa_output::<FeeError>(Some(ovk.clone()), recipient, amount, asset, MemoBytes::empty()).unwrap();

    // Add change output
    let change_amount = total_inputs_amount - amount;

    if change_amount != 0 {
        tx.add_orchard_zsa_output::<FeeError>(Some(ovk), sender, change_amount, asset, MemoBytes::empty()).unwrap();
    }

    build_tx(tx)
}

/// Create a burn transaction
pub fn create_burn_transaction(arsonist: Address, amount: u64, asset: AssetBase, wallet: &mut Wallet) -> Transaction {
    info!("Burn {} zatoshi", amount);

    let mut tx = create_tx(wallet);

    // Add inputs
    let inputs = wallet.select_spendable_notes(arsonist, amount, Some(asset));
    let total_inputs_amount = inputs.iter().fold(0, |acc, input| acc + input.note.value().inner());

    info!("Total inputs amount: {}, amount to burn: {}", total_inputs_amount, amount);

    inputs.into_iter().for_each(|input| tx.add_orchard_zsa_spend::<FeeError>(input.sk, input.note, input.merkle_path).unwrap());

    // Add main transfer output
    tx.add_burn::<FeeError>(NoteValue::from_raw(amount).into(), asset).unwrap();

    // Add change output if needed
    let change_amount = total_inputs_amount - amount;
    if change_amount != 0 {
        let ovk = wallet.orchard_ovk();
        tx.add_orchard_zsa_output::<FeeError>(Some(ovk), arsonist, change_amount, asset, MemoBytes::empty()).unwrap();
    }

    build_tx(tx)
}

/// Create a transaction that issues a new asset
pub fn create_issue_transaction(recipient: Address, amount: u64, asset_desc: String, wallet: &mut Wallet, rpc: &mut dyn RpcClient)  -> Transaction {
    info!("Issue {} asset", amount);
    let mut tx = create_tx(wallet);
    tx.init_issue_bundle::<FeeError>(wallet.issuance_key(), asset_desc, recipient, NoteValue::from_raw(amount)).unwrap();
    build_tx(tx)
}


/// Convert a block template and a list of transactions into a block proposal
pub fn template_into_proposal(block_template: BlockTemplate, mut txs: Vec<Transaction>) -> BlockProposal {

    let coinbase = Transaction::read(hex::decode(block_template.coinbase_txn.data).unwrap().as_slice(), zcash_primitives::consensus::BranchId::Nu5).unwrap();

    let mut txs_with_coinbase = vec![coinbase];
    txs_with_coinbase.append(&mut txs);

    let merkle_root = if txs_with_coinbase.len() == 1 {
        // only coinbase tx is present, no need to calculate
        crate::components::rpc_client::decode_hex(block_template.default_roots.merkle_root)
    } else {
        txs_with_coinbase.iter().map(|tx| { tx.txid().0 }).collect::<Root>().0
    };

    let auth_data_root = txs_with_coinbase.iter().map(|tx| {
        if tx.version().has_orchard() {
            let bytes: [u8;32] = <[u8; 32]>::try_from(tx.auth_commitment().as_bytes()).unwrap();
            bytes
        } else {
            AUTH_COMMITMENT_PLACEHOLDER
        }
    }).collect::<AuthDataRoot>();

    let hash_block_commitments = block_commitment_from_parts(
        crate::components::rpc_client::decode_hex(block_template.default_roots.chain_history_root),
        auth_data_root.0,
    );

    let block_header_data = BlockHeaderData {
        version: block_template.version as i32,
        prev_block: BlockHash(crate::components::rpc_client::decode_hex(block_template.previous_block_hash)),
        merkle_root: merkle_root,
        final_sapling_root: hash_block_commitments,
        time: block_template.cur_time,
        bits: u32::from_str_radix(block_template.bits.as_str(), 16).unwrap(),
        nonce: [2; 32], // Currently PoW is switched off in Zebra
        solution: Vec::from([0; 1344]), // Currently PoW is switched off in Zebra
    };

    let header = BlockHeader::from_data(block_header_data).unwrap();

    BlockProposal {
        header,
        transactions: txs_with_coinbase,
    }
}

fn create_tx(wallet: &Wallet) -> Builder<TestNetwork, OsRng> {
    let tx = Builder::new(TEST_NETWORK, /*wallet.last_block_height().unwrap()*/ BlockHeight::from_u32(1_842_421), wallet.orchard_anchor());
    tx
}

fn build_tx(tx: Builder<TestNetwork, OsRng>) -> Transaction {
    let fee_rule = &FeeRule::non_standard(Amount::from_u64(0).unwrap(), 20, 150, 34).unwrap();
    let prover = LocalTxProver::with_default_location().unwrap();
    let (tx, _) = tx.build(&prover, fee_rule).unwrap();
    info!("Build tx: {}", tx.txid());
    tx
}