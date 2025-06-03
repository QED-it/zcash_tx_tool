//! Main entry point for the application

#![deny(warnings, missing_docs, trivial_casts, unused_qualifications)]
#![forbid(unsafe_code)]

use zcash_tx_tool::application::APP;

/// Boot the application
fn main() {
    println!("Git tag: {}", env!("GIT_TAG"));
    println!("Git commit: {}", env!("GIT_COMMIT"));
    abscissa_core::boot(&APP);
}
