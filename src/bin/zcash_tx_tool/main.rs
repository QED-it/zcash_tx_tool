//! Main entry point for the application

#![deny(warnings, missing_docs, trivial_casts, unused_qualifications)]
#![forbid(unsafe_code)]

use zcash_tx_tool::application::APP;
use std::fs;

/// Boot the application
fn main() {
    print_version_info();
    abscissa_core::boot(&APP);
}

/// Print version information at startup
fn print_version_info() {
    eprintln!("=== ZCash TX Tool Version Information ===");

    if let Ok(version_info) = fs::read_to_string("/app/version_info.env") {
        for line in version_info.lines() {
            if let Some((key, value)) = line.split_once('=') {
                match key {
                    "GIT_COMMIT" => eprintln!("Git Commit: {}", value),
                    "GIT_TAG" => eprintln!("Git Tag: {}", value),
                    "DOCKERFILE_HASH" => eprintln!("Dockerfile Hash: {}", value),
                    _ => {}
                }
            }
        }
    } else {
        eprintln!("Version information not available (not running in Docker)");
    }

    eprintln!("=========================================");
}
