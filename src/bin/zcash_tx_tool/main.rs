//! Main entry point for the application

#![deny(warnings, missing_docs, trivial_casts, unused_qualifications)]
#![forbid(unsafe_code)]

use zcash_tx_tool::application::APP;

/// Boot the application
fn main() {
    println!("Git tag: {}", option_env!("GIT_TAG").unwrap_or("none"));
    println!("Git commit: {}", env!("GIT_COMMIT"));
    println!("Dockerfile hash: {}", env!("DOCKERFILE_HASH"));
    abscissa_core::boot(&APP);
}
