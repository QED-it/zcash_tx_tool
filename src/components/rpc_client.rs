pub mod reqwest;
pub mod mock;

use std::convert::TryInto;
use std::error::Error;
use std::io::Write;
use std::{io, vec};
use zcash_encoding::{CompactSize, Vector};
use zcash_primitives::block::{BlockHash, BlockHeader, BlockHeaderData};
use zcash_primitives::transaction::{Transaction, TxId};
use crate::model::Block;


pub const NODE_URL: &str = "http://127.0.0.1:18232/"; // TODO get from config

pub trait RpcClient {
    fn get_best_block_hash(&self) -> Result<BlockHash, Box<dyn Error>>;
    fn get_block(&self, height: u32) -> Result<Block, Box<dyn Error>>;
    fn send_transaction(&mut self, tx: Transaction) -> Result<TxId, Box<dyn Error>>;
    fn get_transaction(&self, txid: &TxId, block_id: &BlockHash) -> Result<Transaction, Box<dyn Error>>;
    fn get_block_template(&self) -> Result<BlockTemplate, Box<dyn Error>>;
    fn submit_block(&self, block: BlockProposal) -> Result<Option<String>, Box<dyn Error>>;
}

/// =========================== Messages ===========================

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GetBlock {
        /// The hash of the requested block in hex.
        hash: String,

        /// The number of confirmations of this block in the best chain,
        /// or -1 if it is not in the best chain.
        confirmations: i64,

        /// The height of the requested block.
        #[serde(skip_serializing_if = "Option::is_none")]
        height: Option<u32>,

        /// List of transaction IDs in block order, hex-encoded.
        //
        // TODO: use a typed Vec<transaction::Hash> here
        tx: Vec<String>
}


/// A serialized `getblocktemplate` RPC response in template mode.
#[derive(Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BlockTemplate {
    /// The getblocktemplate RPC capabilities supported by Zebra.
    ///
    /// At the moment, Zebra does not support any of the extra capabilities from the specification:
    /// - `proposal`: <https://en.bitcoin.it/wiki/BIP_0023#Block_Proposal>
    /// - `longpoll`: <https://en.bitcoin.it/wiki/BIP_0022#Optional:_Long_Polling>
    /// - `serverlist`: <https://en.bitcoin.it/wiki/BIP_0023#Logical_Services>
    ///
    /// By the above, Zebra will always return an empty vector here.
    pub capabilities: Vec<String>,

    /// The version of the block format.
    /// Always 4 for new Zcash blocks.
    pub version: u32,

    /// The hash of the previous block.
    #[serde(rename = "previousblockhash")]
    pub previous_block_hash: String,

    /// The block commitment for the new block's header.
    ///
    /// Same as [`DefaultRoots.block_commitments_hash`], see that field for details.
    #[serde(rename = "blockcommitmentshash")]
    pub block_commitments_hash: String,

    /// Legacy backwards-compatibility header root field.
    ///
    /// Same as [`DefaultRoots.block_commitments_hash`], see that field for details.
    #[serde(rename = "lightclientroothash")]
    pub light_client_root_hash: String,

    /// Legacy backwards-compatibility header root field.
    ///
    /// Same as [`DefaultRoots.block_commitments_hash`], see that field for details.
    #[serde(rename = "finalsaplingroothash")]
    pub final_sapling_root_hash: String,

    /// The block header roots for [`GetBlockTemplate.transactions`].
    ///
    /// If the transactions in the block template are modified, these roots must be recalculated
    /// [according to the specification](https://zcash.github.io/rpc/getblocktemplate.html).
    #[serde(rename = "defaultroots")]
    pub default_roots: DefaultRoots,

    /// The non-coinbase transactions selected for this block template.
    pub transactions: Vec<TransactionTemplate>,

    /// The coinbase transaction generated from `transactions` and `height`.
    #[serde(rename = "coinbasetxn")]
    pub coinbase_txn: TransactionTemplate,

    /// An ID that represents the chain tip and mempool contents for this template.
    #[serde(rename = "longpollid")]
    pub long_poll_id: String,

    /// The expected difficulty for the new block displayed in expanded form.
    pub target: String,

    /// > For each block other than the genesis block, nTime MUST be strictly greater than
    /// > the median-time-past of that block.
    ///
    /// <https://zips.z.cash/protocol/protocol.pdf#blockheader>
    #[serde(rename = "mintime")]
    pub min_time: u32,

    /// Hardcoded list of block fields the miner is allowed to change.
    pub mutable: Vec<String>,

    /// A range of valid nonces that goes from `u32::MIN` to `u32::MAX`.
    #[serde(rename = "noncerange")]
    pub nonce_range: String,

    /// Max legacy signature operations in the block.
    #[serde(rename = "sigoplimit")]
    pub sigop_limit: u64,

    /// Max block size in bytes
    #[serde(rename = "sizelimit")]
    pub size_limit: u64,

    /// > the current time as seen by the server (recommended for block time).
    /// > note this is not necessarily the system clock, and must fall within the mintime/maxtime rules
    ///
    /// <https://en.bitcoin.it/wiki/BIP_0022#Block_Template_Request>
    #[serde(rename = "curtime")]
    pub cur_time: u32,

    /// The expected difficulty for the new block displayed in compact form.
    pub bits: String,

    /// The height of the next block in the best chain.
    // Optional TODO: use Height type, but check that deserialized heights are within Height::MAX
    pub height: u32,

    /// > the maximum time allowed
    ///
    /// <https://en.bitcoin.it/wiki/BIP_0023#Mutations>
    ///
    /// Zebra adjusts the minimum and current times for testnet minimum difficulty blocks,
    /// so we need to tell miners what the maximum valid time is.
    ///
    /// This field is not in `zcashd` or the Zcash RPC reference yet.
    ///
    /// Currently, some miners just use `min_time` or `cur_time`. Others calculate `max_time` from the
    /// fixed 90 minute consensus rule, or a smaller fixed interval (like 1000s).
    /// Some miners don't check the maximum time. This can cause invalid blocks after network downtime,
    /// a significant drop in the hash rate, or after the testnet minimum difficulty interval.
    #[serde(rename = "maxtime")]
    pub max_time: u32,

    /// > only relevant for long poll responses:
    /// > indicates if work received prior to this response remains potentially valid (default)
    /// > and should have its shares submitted;
    /// > if false, the miner may wish to discard its share queue
    ///
    /// <https://en.bitcoin.it/wiki/BIP_0022#Optional:_Long_Polling>
    ///
    /// This field is not in `zcashd` or the Zcash RPC reference yet.
    ///
    /// In Zebra, `submit_old` is `false` when the tip block changed or max time is reached,
    /// and `true` if only the mempool transactions have changed.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[serde(rename = "submitold")]
    pub submit_old: Option<bool>,
}

/// The block header roots for [`GetBlockTemplate.transactions`].
///
/// If the transactions in the block template are modified, these roots must be recalculated
/// [according to the specification](https://zcash.github.io/rpc/getblocktemplate.html).
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DefaultRoots {
    /// The merkle root of the transaction IDs in the block.
    /// Used in the new block's header.
    #[serde(rename = "merkleroot")]
    pub merkle_root: String,

    /// The root of the merkle mountain range of the chain history roots from the last network upgrade to the previous block.
    /// Unlike the other roots, this not cover any data from this new block, only from previous blocks.
    #[serde(rename = "chainhistoryroot")]
    pub chain_history_root: String,

    /// The merkle root of the authorizing data hashes of the transactions in the new block.
    #[serde(rename = "authdataroot")]
    pub auth_data_root: String,

    /// The block commitment for the new block's header.
    /// This hash covers `chain_history_root` and `auth_data_root`.
    ///
    /// `merkle_root` has its own field in the block header.
    #[serde(rename = "blockcommitmentshash")]
    pub block_commitments_hash: String,
}

/// Transaction data and fields needed to generate blocks using the `getblocktemplate` RPC.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TransactionTemplate {
    /// The hex-encoded serialized data for this transaction.
    pub data: String,

    /// The transaction ID of this transaction.
    pub(crate) hash: String,

    /// The authorizing data digest of a v5 transaction, or a placeholder for older versions.
    #[serde(rename = "authdigest")]
    pub(crate) auth_digest: String,

    /// The transactions in this block template that this transaction depends upon.
    /// These are 1-based indexes in the `transactions` list.
    ///
    /// Zebra's mempool does not support transaction dependencies, so this list is always empty.
    ///
    /// We use `u16` because 2 MB blocks are limited to around 39,000 transactions.
    pub(crate) depends: Vec<u16>,

    /// The fee for this transaction.
    ///
    /// Non-coinbase transactions must be `NonNegative`.
    /// The Coinbase transaction `fee` is the negative sum of the fees of the transactions in
    /// the block, so their fee must be `NegativeOrZero`.
    pub(crate) fee: i64,

    /// The number of transparent signature operations in this transaction.
    pub(crate) sigops: u64,

    /// Is this transaction required in the block?
    ///
    /// Coinbase transactions are required, all other transactions are not.
    pub(crate) required: bool,
}

pub struct BlockProposal {
    /// The block header, containing block metadata.
    pub header: BlockHeader,
    /// The block transactions.
    pub transactions: Vec<Transaction>,
}

impl BlockProposal {
    pub fn write<W: Write>(&self, mut writer: W) -> io::Result<()> {
        self.header.write(&mut writer)?;
        if !self.transactions.is_empty() {
            Vector::write(&mut writer, self.transactions.as_slice(), |w, tx| {
                tx.write(w)
            })?;
        } else {
            CompactSize::write(&mut writer, 0)?;
        }
        Ok(())
    }
}

pub fn template_into_proposal(block_template: BlockTemplate) -> BlockProposal {

    let coinbase = Transaction::read(hex::decode(block_template.coinbase_txn.data).unwrap().as_slice(), zcash_primitives::consensus::BranchId::Nu5).unwrap();

    let mut prev_block_hash_vec = hex::decode(block_template.previous_block_hash).unwrap();
    prev_block_hash_vec.reverse();
    let prev_block_hash_bytes: [u8; 32] = prev_block_hash_vec.try_into().unwrap();

    let mut history_root_vec = hex::decode(block_template.default_roots.chain_history_root).unwrap();
    history_root_vec.reverse();
    let history_root_bytes: [u8; 32] = history_root_vec.try_into().unwrap();

    let block_header_data = BlockHeaderData {
        version: block_template.version as i32,
        prev_block: BlockHash(prev_block_hash_bytes),
        merkle_root: coinbase.txid().0,
        final_sapling_root: history_root_bytes,
        time: block_template.cur_time,
        bits: u32::from_str_radix(block_template.bits.as_str(), 16).unwrap(),
        nonce: [2; 32],
        solution: Vec::from([0; 1344]),
    };

    let header = BlockHeader::from_data(block_header_data).unwrap();

    BlockProposal {
        header,
        transactions: vec![coinbase],
    }
}