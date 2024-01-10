use std::collections::BTreeMap;
use std::io;
use std::io::ErrorKind;
use std::error::Error;
use rand::RngCore;
use rand::rngs::OsRng;
use zcash_primitives::block::BlockHash;
use zcash_primitives::consensus::{BlockHeight, BranchId};
use zcash_primitives::transaction::{Transaction, TxId};
use crate::components::rpc_client::{BlockProposal, BlockTemplate, RpcClient};
use crate::model::Block;


pub struct MockZcashNode {
    blockchain: Vec<Block>, // TODO forks
    transactions: BTreeMap<TxId, String>
}

impl MockZcashNode {
    pub fn new() -> Self {

        let genesis = Block {
            hash: BlockHash::from_slice(&[17; 32]),
            height: BlockHeight::from_u32(0),
            confirmations: 0,
            tx_ids: vec![],
            previous_block_hash: BlockHash::from_slice(&[0; 32])
        };

        let second = Block {
            hash: BlockHash::from_slice(&[34; 32]),
            height: BlockHeight::from_u32(1),
            confirmations: 0,
            tx_ids: vec![],
            previous_block_hash: BlockHash::from_slice(&[1; 32])
        };


        Self {
            blockchain: vec![genesis, second],
            transactions: BTreeMap::new()
        }
    }
}

impl RpcClient for MockZcashNode {

    fn get_best_block_hash(&self) -> Result<BlockHash, Box<dyn Error>> {
        self.blockchain.last().ok_or(io::Error::new(ErrorKind::NotFound, "Block not found").into()).map(|b| b.hash)
    }

    fn get_block(&self, height: u32) -> Result<Block, Box<dyn Error>> {
        self.blockchain.get(height as usize).ok_or(io::Error::new(ErrorKind::NotFound, "Block not found").into()).map(|b| b.clone())
    }

    fn send_transaction(&mut self, tx: Transaction) -> Result<TxId, Box<dyn Error>> {
        let txid = tx.txid();
        let mut tx_bytes = vec![];
        tx.write(&mut tx_bytes).unwrap();
        self.transactions.insert(txid, hex::encode(tx_bytes));
        // We create block per transaction for now
        let mut block_hash: [u8; 32] = [0; 32]; // TODO use real hash
        OsRng::default().fill_bytes(block_hash.as_mut_slice());

        self.blockchain.push(Block {
            hash: BlockHash::from_slice(block_hash.as_slice()),
            height: BlockHeight::from_u32(self.blockchain.len() as u32),
            confirmations: 0,
            tx_ids: vec![txid],
            previous_block_hash: self.blockchain.last().unwrap().hash
        });
        Ok(txid)
    }

    fn get_transaction(&self, txid: &TxId, block_id: &BlockHash) -> Result<Transaction, Box<dyn Error>> {
        self.transactions.get(txid).ok_or(io::Error::new(ErrorKind::NotFound, "Transaction not found").into()).map(|tx_string| {
            Transaction::read(&hex::decode(tx_string).unwrap()[..], BranchId::Nu5).unwrap()
        })
    }

    fn get_block_template(&self) -> Result<BlockTemplate, Box<dyn Error>> {
        todo!()
    }

    fn submit_block(&self, block: BlockProposal) -> Result<Option<String>, Box<dyn Error>> {
        todo!()
    }
}