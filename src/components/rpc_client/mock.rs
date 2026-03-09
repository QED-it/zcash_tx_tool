use crate::components::rpc_client::{BlockProposal, BlockTemplate, RpcClient};
use crate::model::Block;
use crate::prelude::info;
use rand::rngs::OsRng;
use rand::RngCore;
use std::collections::BTreeMap;
use std::error::Error;
use std::io;
use std::io::ErrorKind;
use zcash_primitives::block::BlockHash;
use zcash_primitives::consensus::{BlockHeight, BranchId};
use zcash_primitives::transaction::{Transaction, TxId};

pub struct MockZcashNode {
    blockchain: Vec<Block>,
    transactions: BTreeMap<TxId, String>,
}

impl MockZcashNode {
    pub fn new() -> Self {
        let genesis = Block {
            hash: BlockHash::from_slice(&[17; 32]),
            height: BlockHeight::from_u32(0),
            confirmations: 0,
            tx_ids: vec![],
            previous_block_hash: BlockHash::from_slice(&[0; 32]),
        };

        Self {
            blockchain: vec![genesis],
            transactions: BTreeMap::new(),
        }
    }
}

impl Default for MockZcashNode {
    fn default() -> Self {
        Self::new()
    }
}

impl RpcClient for MockZcashNode {
    fn get_best_block_hash(&self) -> Result<BlockHash, Box<dyn Error>> {
        self.blockchain
            .last()
            .ok_or(io::Error::new(ErrorKind::NotFound, "Block not found").into())
            .map(|b| b.hash)
    }

    fn get_block(&self, height: u32) -> Result<Block, Box<dyn Error>> {
        self.blockchain
            .get(height as usize)
            .ok_or(io::Error::new(ErrorKind::NotFound, "Block not found").into())
            .cloned()
    }

    fn send_transaction(&mut self, tx: Transaction) -> Result<TxId, Box<dyn Error>> {
        let txid = tx.txid();
        let mut tx_bytes = vec![];
        tx.write(&mut tx_bytes).unwrap();
        self.transactions.insert(txid, hex::encode(tx_bytes));
        // We create block per transaction for now
        let mut block_hash: [u8; 32] = [0; 32];
        OsRng.fill_bytes(block_hash.as_mut_slice());

        self.blockchain.push(Block {
            hash: BlockHash::from_slice(block_hash.as_slice()),
            height: BlockHeight::from_u32(self.blockchain.len() as u32),
            confirmations: 0,
            tx_ids: vec![txid],
            previous_block_hash: self.blockchain.last().unwrap().hash,
        });
        Ok(txid)
    }

    fn get_transaction(&self, txid: &TxId) -> Result<Transaction, Box<dyn Error>> {
        self.transactions
            .get(txid)
            .ok_or(io::Error::new(ErrorKind::NotFound, "Transaction not found").into())
            .map(|tx_string| {
                Transaction::read(&hex::decode(tx_string).unwrap()[..], BranchId::Nu6).unwrap()
            })
    }

    fn get_block_template(&self) -> Result<BlockTemplate, Box<dyn Error>> {
        Ok(BlockTemplate::new(self.blockchain.len() as u32))
    }

    fn submit_block(&mut self, block: BlockProposal) -> Result<Option<String>, Box<dyn Error>> {
        let mut block_bytes = vec![];
        block.write(&mut block_bytes).unwrap();
        let serialized_block = hex::encode(block_bytes);

        info!("Submit block \"{}\"", serialized_block);

        let len: usize = self.blockchain.len();

        // Step 1: Collect the encoded transactions and their IDs outside the closure
        let transactions_to_insert: Vec<(TxId, String)> = block
            .transactions
            .iter()
            .map(|tx| {
                let mut tx_bytes = vec![];
                tx.write(&mut tx_bytes).unwrap();
                (tx.txid(), hex::encode(tx_bytes))
            })
            .collect();

        // Step 2: Create the new block and push it to the blockchain
        self.blockchain.push(Block {
            hash: BlockHash::from_slice(&[len as u8; 32]),
            height: BlockHeight::from_u32(len as u32),
            confirmations: 0,
            tx_ids: block.transactions.iter().map(|tx| tx.txid()).collect(),
            previous_block_hash: BlockHash::from_slice(&[len as u8; 32]),
        });

        // Step 3: Insert the collected transactions into the self.transactions map
        for (txid, encoded_tx) in transactions_to_insert {
            self.transactions.insert(txid, encoded_tx);
        }

        Ok(Some("".to_string()))
    }
}
