//! ZsaWallet Config
//!
//! See instructions in `commands.rs` to specify the path to your
//! application's configuration file and/or command-line options
//! for specifying it.

use serde::{Deserialize, Serialize};

/// ZsaWallet Configuration
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ZsaWalletConfig {
    /// An example configuration section
    pub wallet: WalletSection,
}

/// Default configuration settings.
///
/// Note: if your needs are as simple as below, you can
/// use `#[derive(Default)]` on ZsaWalletConfig instead.
impl Default for ZsaWalletConfig {
    fn default() -> Self {
        Self {
            wallet: WalletSection::default(),
        }
    }
}

/// Wallet configuration
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WalletSection {
    /// Wallet seed phrase as defined in BIP-39
    pub seed_phrase: String,
}

impl Default for WalletSection {
    fn default() -> Self {
        Self {
            seed_phrase: String::new(),
        }
    }
}
