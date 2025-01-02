//! Application Config
//!
//! See instructions in `commands.rs` to specify the path to your
//! application's configuration file and/or command-line options
//! for specifying it.

use serde::{Deserialize, Serialize};
use std::env;

/// Application Configuration
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct AppConfig {
    pub wallet: WalletSection,
    pub network: NetworkConfig,
    pub chain: ChainConfig,
}

/// Default configuration settings.
///
/// Note: if your needs are as simple as below, you can
/// use `#[derive(Default)]` on AppConfig instead.
impl Default for AppConfig {
    fn default() -> Self {
        Self {
            wallet: WalletSection::default(),
            network: NetworkConfig::default(),
            chain: ChainConfig::default(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct WalletSection {
    /// Wallet seed phrase as defined in BIP-39
    pub seed_phrase: String,
    /// Miner seed phrase as defined in BIP-39
    pub miner_seed_phrase: String,
}

impl Default for WalletSection {
    fn default() -> Self {
        Self {
            seed_phrase: "fabric dilemma shift time border road fork license among uniform early laundry caution deer stamp".to_string(), // tmLTZegcJN5zaufWQBARHkvqC62mTumm3jR
            miner_seed_phrase: "fabric dilemma shift time border road fork license among uniform early laundry caution deer stamp".to_string(), // tmLTZegcJN5zaufWQBARHkvqC62mTumm3jR
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct NetworkConfig {
    pub node_address: String,
    pub node_port: u32,
    pub protocol: String,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        let node_address = env::var("ZCASH_NODE_ADDRESS").unwrap_or_else(|_| "127.0.0.1".to_string());
        let node_port = env::var("ZCASH_NODE_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(18232);
        let protocol = env::var("ZCASH_NODE_PROTOCOL").unwrap_or_else(|_| "http".to_string());

        println!(
            "Using NetworkConfig: node_address = {}, node_port = {}, protocol = {}",
            node_address, node_port, protocol
        );

        Self {
            node_address,
            node_port,
            protocol,
        }
    }
}

impl NetworkConfig {
    pub fn node_url(&self) -> String {
        format!(
            "{}://{}:{}",
            self.protocol, self.node_address, self.node_port
        )
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct ChainConfig {
    pub nu5_activation_height: u32,
    pub v6_activation_height: u32,
}

impl Default for ChainConfig {
    fn default() -> Self {
        Self {
            nu5_activation_height: 1, // NU5 activation height for regtest, should be in sync with node's chain params
            v6_activation_height: 1, // V6 activation height for regtest, should be in sync with node's chain params
        }
    }
}
