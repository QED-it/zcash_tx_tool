use zcash_primitives::block::BlockHash;
use zcash_primitives::consensus::BlockHeight;
use zcash_primitives::transaction::TxId;

pub struct Block {
    pub hash: BlockHash,
    pub height: BlockHeight,
    pub confirmations: i64,
    pub tx_ids: Vec<TxId>,
    pub previous_block_hash: BlockHash
}