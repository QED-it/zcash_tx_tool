//! Main entry point for the application

#![deny(warnings, missing_docs, trivial_casts, unused_qualifications)]
#![forbid(unsafe_code)]

use zcash_tx_tool::application::APP;

/// Boot the application
fn main() {
    eprintln!("Git tag: {}", option_env!("GIT_TAG").unwrap_or("none"));
    eprintln!("Git commit: {}", env!("GIT_COMMIT"));
    abscissa_core::boot(&APP);
}
