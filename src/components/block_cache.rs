use zebra_chain::block::Block;

pub struct BlockCache {
    pub latest_height: u64,  // height of the latest obtained block
    pub latest_hash: String, // hash of the latest obtained block
}

impl BlockCache {

    pub fn new() -> Self {
        Self {
            latest_height: 0,
            latest_hash: "latest_hash".to_string(),
        }
    }

    pub fn add(&mut self, block: &Block) {
        self.latest_height = block.height;
        self.latest_hash = block.hash.clone();
    }

    pub fn reorg(&mut self, height: u64) {
        todo!()
        // self.latest_height = height;
        // self.latest_hash = get_block_hash(height);
        // drop_all_blocks_after_height()
    }
}