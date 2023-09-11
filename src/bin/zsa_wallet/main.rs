//! Main entry point for ZsaWallet

#![deny(warnings, missing_docs, trivial_casts, unused_qualifications)]
#![forbid(unsafe_code)]

use zsa_wallet::application::APP;

/// Boot ZsaWallet
fn main() {
    abscissa_core::boot(&APP);
}
