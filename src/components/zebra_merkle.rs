use std::convert::TryInto;
use std::iter::FromIterator;
/// Partially copied from `zebra/zebra-chain/src/block/merkle.rs`
use std::{fmt, iter};

use hex::{FromHex, ToHex};
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Root(pub [u8; 32]);

impl fmt::Debug for Root {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Root").field(&hex::encode(self.0)).finish()
    }
}

impl From<[u8; 32]> for Root {
    fn from(hash: [u8; 32]) -> Self {
        Root(hash)
    }
}

impl From<Root> for [u8; 32] {
    fn from(hash: Root) -> Self {
        hash.0
    }
}

impl Root {
    /// Return the hash bytes in big-endian byte-order suitable for printing out byte by byte.
    ///
    /// Zebra displays transaction and block hashes in big-endian byte-order,
    /// following the u256 convention set by Bitcoin and zcashd.
    pub fn bytes_in_display_order(&self) -> [u8; 32] {
        let mut reversed_bytes = self.0;
        reversed_bytes.reverse();
        reversed_bytes
    }

    /// Convert bytes in big-endian byte-order into a [`merkle::Root`](crate::block::merkle::Root).
    ///
    /// Zebra displays transaction and block hashes in big-endian byte-order,
    /// following the u256 convention set by Bitcoin and zcashd.
    pub fn from_bytes_in_display_order(bytes_in_display_order: &[u8; 32]) -> Root {
        let mut internal_byte_order = *bytes_in_display_order;
        internal_byte_order.reverse();

        Root(internal_byte_order)
    }
}

impl ToHex for &Root {
    fn encode_hex<T: FromIterator<char>>(&self) -> T {
        self.bytes_in_display_order().encode_hex()
    }

    fn encode_hex_upper<T: FromIterator<char>>(&self) -> T {
        self.bytes_in_display_order().encode_hex_upper()
    }
}

impl ToHex for Root {
    fn encode_hex<T: FromIterator<char>>(&self) -> T {
        (&self).encode_hex()
    }

    fn encode_hex_upper<T: FromIterator<char>>(&self) -> T {
        (&self).encode_hex_upper()
    }
}

impl FromHex for Root {
    type Error = <[u8; 32] as FromHex>::Error;

    fn from_hex<T: AsRef<[u8]>>(hex: T) -> Result<Self, Self::Error> {
        let mut hash = <[u8; 32]>::from_hex(hex)?;
        hash.reverse();

        Ok(hash.into())
    }
}

impl FromIterator<[u8; 32]> for Root {
    /// # Panics
    ///
    /// When there are no transactions in the iterator.
    /// This is impossible, because every block must have a coinbase transaction.
    fn from_iter<I>(hashes: I) -> Self
    where
        I: IntoIterator<Item = [u8; 32]>,
    {
        let mut hashes = hashes.into_iter().collect::<Vec<_>>();
        while hashes.len() > 1 {
            hashes = hashes
                .chunks(2)
                .map(|chunk| match chunk {
                    [h1, h2] => hash(h1, h2),
                    [h1] => hash(h1, h1),
                    _ => unreachable!("chunks(2)"),
                })
                .collect();
        }
        Self(hashes[0])
    }
}

fn hash(h1: &[u8; 32], h2: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(h1);
    hasher.update(h2);
    let result1 = hasher.finalize();
    let result2 = Sha256::digest(result1);
    let mut buffer = [0u8; 32];
    buffer[0..32].copy_from_slice(&result2[0..32]);
    buffer
}

fn auth_data_hash(h1: &[u8; 32], h2: &[u8; 32]) -> [u8; 32] {
    // > Non-leaf hashes in this tree are BLAKE2b-256 hashes personalized by
    // > the string "ZcashAuthDatHash".
    // https://zips.z.cash/zip-0244#block-header-changes
    blake2b_simd::Params::new()
        .hash_length(32)
        .personal(b"ZcashAuthDatHash")
        .to_state()
        .update(h1)
        .update(h2)
        .finalize()
        .as_bytes()
        .try_into()
        .expect("32 byte array")
}

/// The root of the authorizing data Merkle tree, binding the
/// block header to the authorizing data of the block (signatures, proofs)
/// as defined in [ZIP-244].
///
/// See [`Root`] for an important disclaimer.
///
/// [ZIP-244]: https://zips.z.cash/zip-0244
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct AuthDataRoot(pub(crate) [u8; 32]);

impl fmt::Debug for AuthDataRoot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("AuthRoot")
            .field(&hex::encode(self.0))
            .finish()
    }
}

impl From<[u8; 32]> for AuthDataRoot {
    fn from(hash: [u8; 32]) -> Self {
        AuthDataRoot(hash)
    }
}

impl From<AuthDataRoot> for [u8; 32] {
    fn from(hash: AuthDataRoot) -> Self {
        hash.0
    }
}

impl AuthDataRoot {
    /// Return the hash bytes in big-endian byte-order suitable for printing out byte by byte.
    ///
    /// Zebra displays transaction and block hashes in big-endian byte-order,
    /// following the u256 convention set by Bitcoin and zcashd.
    pub fn bytes_in_display_order(&self) -> [u8; 32] {
        let mut reversed_bytes = self.0;
        reversed_bytes.reverse();
        reversed_bytes
    }

    /// Convert bytes in big-endian byte-order into a [`merkle::AuthDataRoot`](crate::block::merkle::AuthDataRoot).
    ///
    /// Zebra displays transaction and block hashes in big-endian byte-order,
    /// following the u256 convention set by Bitcoin and zcashd.
    pub fn from_bytes_in_display_order(bytes_in_display_order: &[u8; 32]) -> AuthDataRoot {
        let mut internal_byte_order = *bytes_in_display_order;
        internal_byte_order.reverse();

        AuthDataRoot(internal_byte_order)
    }
}

impl ToHex for &AuthDataRoot {
    fn encode_hex<T: FromIterator<char>>(&self) -> T {
        self.bytes_in_display_order().encode_hex()
    }

    fn encode_hex_upper<T: FromIterator<char>>(&self) -> T {
        self.bytes_in_display_order().encode_hex_upper()
    }
}

impl ToHex for AuthDataRoot {
    fn encode_hex<T: FromIterator<char>>(&self) -> T {
        (&self).encode_hex()
    }

    fn encode_hex_upper<T: FromIterator<char>>(&self) -> T {
        (&self).encode_hex_upper()
    }
}

impl FromHex for AuthDataRoot {
    type Error = <[u8; 32] as FromHex>::Error;

    fn from_hex<T: AsRef<[u8]>>(hex: T) -> Result<Self, Self::Error> {
        let mut hash = <[u8; 32]>::from_hex(hex)?;
        hash.reverse();

        Ok(hash.into())
    }
}

/// The placeholder used for the [`AuthDigest`](transaction::AuthDigest) of pre-v5 transactions.
///
/// # Consensus
///
/// > For transaction versions before v5, a placeholder value consisting
/// > of 32 bytes of 0xFF is used in place of the authorizing data commitment.
/// > This is only used in the tree committed to by hashAuthDataRoot.
///
/// <https://zips.z.cash/zip-0244#authorizing-data-commitment>
pub const AUTH_COMMITMENT_PLACEHOLDER: [u8; 32] = [0xFFu8; 32];

impl FromIterator<[u8; 32]> for AuthDataRoot {
    fn from_iter<I>(hashes: I) -> Self
    where
        I: IntoIterator<Item = [u8; 32]>,
    {
        let mut hashes = hashes.into_iter().collect::<Vec<_>>();
        // > This new commitment is named hashAuthDataRoot and is the root of a
        // > binary Merkle tree of transaction authorizing data commitments [...]
        // > padded with leaves having the "null" hash value [0u8; 32].
        // https://zips.z.cash/zip-0244#block-header-changes
        // Pad with enough leaves to make the tree full (a power of 2).
        let pad_count = hashes.len().next_power_of_two() - hashes.len();
        hashes.extend(iter::repeat([0u8; 32]).take(pad_count));
        assert!(hashes.len().is_power_of_two());

        while hashes.len() > 1 {
            hashes = hashes
                .chunks(2)
                .map(|chunk| match chunk {
                    [h1, h2] => auth_data_hash(h1, h2),
                    _ => unreachable!("number of nodes is always even since tree is full"),
                })
                .collect();
        }

        Self(hashes[0])
    }
}

/// Compute the block commitment from the history tree root and the
/// authorization data root, as specified in [ZIP-244].
///
/// `history_tree_root` is the root of the history tree up to and including
/// the *previous* block.
/// `auth_data_root` is the root of the Merkle tree of authorizing data
/// commmitments of each transaction in the *current* block.
///
///  [ZIP-244]: https://zips.z.cash/zip-0244#block-header-changes
pub fn block_commitment_from_parts(
    history_tree_root: [u8; 32],
    auth_data_root: [u8; 32],
) -> [u8; 32] {
    // > The value of this hash [hashBlockCommitments] is the BLAKE2b-256 hash personalized
    // > by the string "ZcashBlockCommit" of the following elements:
    // >   hashLightClientRoot (as described in ZIP 221)
    // >   hashAuthDataRoot    (as described below)
    // >   terminator          [0u8;32]
    let hash_block_commitments: [u8; 32] = blake2b_simd::Params::new()
        .hash_length(32)
        .personal(b"ZcashBlockCommit")
        .to_state()
        .update(&history_tree_root)
        .update(&auth_data_root)
        .update(&[0u8; 32])
        .finalize()
        .as_bytes()
        .try_into()
        .expect("32 byte array");
    hash_block_commitments
}
