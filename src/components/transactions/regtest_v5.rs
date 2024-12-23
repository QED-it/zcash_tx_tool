use zcash_primitives::consensus::{BlockHeight, NetworkType, NetworkUpgrade, Parameters};

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub struct RegtestNetworkV5;

pub const REGTEST_NETWORK_V5: RegtestNetworkV5 = RegtestNetworkV5;

impl Parameters for RegtestNetworkV5 {
    fn network_type(&self) -> NetworkType {
        NetworkType::Regtest
    }

    fn activation_height(&self, nu: NetworkUpgrade) -> Option<BlockHeight> {
        match nu {
            NetworkUpgrade::Overwinter => Some(BlockHeight::from_u32(1)),
            NetworkUpgrade::Sapling => Some(BlockHeight::from_u32(1)),
            NetworkUpgrade::Blossom => Some(BlockHeight::from_u32(1)),
            NetworkUpgrade::Heartwood => Some(BlockHeight::from_u32(1)),
            NetworkUpgrade::Canopy => Some(BlockHeight::from_u32(1)),
            NetworkUpgrade::Nu5 => Some(BlockHeight::from_u32(1)),
            NetworkUpgrade::Nu6 => None,
            NetworkUpgrade::Nu7 => None,
            #[cfg(zcash_unstable = "zfuture")]
            NetworkUpgrade::ZFuture => None,
        }
    }
}
