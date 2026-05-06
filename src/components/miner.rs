//! The transparent keypair whose address Zebra is configured to pay coinbase
//! rewards to.
//!
//! In regtest, Zebra mines blocks and pays the coinbase to a single hard-coded
//! address; the tx-tool holds the matching secret key here so it can *spend*
//! those coinbase outputs (i.e., shield them into the Orchard wallet). The
//! tool itself never produces blocks — that's Zebra's job — and `MinerKey` is
//! not used outside the t→z shielding path.

use bip0039::Mnemonic;
use ripemd::{Digest, Ripemd160};
use secp256k1::{Secp256k1, SecretKey};
use sha2::Sha256;
use zcash_primitives::zip32::AccountId;
use zcash_protocol::consensus::REGTEST_NETWORK;
use zcash_transparent::address::TransparentAddress;
use zcash_transparent::builder::TransparentSigningSet;
use zcash_transparent::keys::NonHardenedChildIndex;

pub struct MinerKey {
    seed: [u8; 64],
}

impl MinerKey {
    pub fn new(seed_phrase: &str) -> Self {
        Self {
            seed: <Mnemonic>::from_phrase(seed_phrase).unwrap().to_seed(""),
        }
    }

    pub(crate) fn address(&self) -> TransparentAddress {
        let pubkey = self
            .secret_key()
            .public_key(&Secp256k1::new())
            .serialize();
        let hash = &Ripemd160::digest(Sha256::digest(pubkey))[..];
        TransparentAddress::PublicKeyHash(hash.try_into().unwrap())
    }

    pub(crate) fn secret_key(&self) -> SecretKey {
        let account = AccountId::try_from(0).unwrap();
        zcash_transparent::keys::AccountPrivKey::from_seed(&REGTEST_NETWORK, &self.seed, account)
            .unwrap()
            .derive_external_secret_key(NonHardenedChildIndex::ZERO)
            .unwrap()
    }

    pub(crate) fn signing_set(&self) -> TransparentSigningSet {
        let mut tss = TransparentSigningSet::new();
        tss.add_key(self.secret_key());
        tss
    }
}
