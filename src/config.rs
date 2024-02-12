//! Application Config
//!
//! See instructions in `commands.rs` to specify the path to your
//! application's configuration file and/or command-line options
//! for specifying it.

use serde::{Deserialize, Serialize};

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
}

impl Default for WalletSection {
    fn default() -> Self {
        Self {
            seed_phrase: "fabric dilemma shift time border road fork license among uniform early laundry caution deer stamp".to_string(), // tmLTZegcJN5zaufWQBARHkvqC62mTumm3jR
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
        Self {
            node_address: "127.0.0.1".to_string(),
            node_port: 18232,
            protocol: "http".to_string(),
        }
    }
}

impl NetworkConfig {
    pub fn node_url(&self) -> String {
        format!("{}://{}:{}", self.protocol, self.node_address, self.node_port)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct ChainConfig {
    pub nu5_activation_height: u32,
}

impl Default for ChainConfig {
    fn default() -> Self {
        Self {
            nu5_activation_height: 1_060_755, // NU5 activation height for shorter chain, should be in sync with node's chain params
        }
    }
}