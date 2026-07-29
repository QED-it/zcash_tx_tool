//! Zcash Tx Tool
//!
//! Application based on the [Abscissa] framework.
//!
//! [Abscissa]: https://github.com/iqlusioninc/abscissa

// Tip: Deny warnings with `RUSTFLAGS="-D warnings"` environment variable in CI

#![forbid(unsafe_code)]
#![warn(rust_2018_idioms, trivial_casts, unused_lifetimes)]

pub mod application;
pub mod commands;
pub mod components;
pub mod config;
pub mod error;
mod model;
pub mod prelude;
mod schema;

/// Subcommands that produce clean, user-facing stdout (the wallet commands
/// and machine-readable outputs); startup banner/config prints are suppressed
/// for them and their default tracing level is reduced to warnings.
pub const QUIET_COMMANDS: &[&str] = &[
    "get-block-data",
    "status",
    "sync",
    "addresses",
    "balance",
    "notes",
    "assets",
    "issue",
    "transfer",
    "burn",
    "finalize",
    "mine",
    "shield",
    "clean",
];

/// Whether the current invocation runs one of the [`QUIET_COMMANDS`].
pub fn is_quiet_invocation() -> bool {
    is_quiet_command(std::env::args().skip(1))
}

/// Whether the subcommand in `args` (the argv tail, without the binary name)
/// is one of the [`QUIET_COMMANDS`].
fn is_quiet_command<I: IntoIterator<Item = String>>(args: I) -> bool {
    subcommand_of(args).is_some_and(|cmd| QUIET_COMMANDS.contains(&cmd.as_str()))
}

/// The subcommand of an invocation: the first positional argument, skipping
/// global flags and the value of `--config`/`-c`.
///
/// Matching only the subcommand (rather than scanning the whole argv) keeps an
/// *argument* that happens to spell a command name — an asset described as
/// "mine", say — from silencing an otherwise noisy subcommand.
fn subcommand_of<I: IntoIterator<Item = String>>(args: I) -> Option<String> {
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if let Some(short) = arg.strip_prefix('-').filter(|a| !a.starts_with('-')) {
            // A short-flag cluster ending in `c` (`-c`, `-vc`) takes the next
            // argument as the config path; `-cpath` carries it inline.
            if short.ends_with('c') {
                args.next();
            }
            continue;
        }
        // Long flags, including `--config=<path>`, carry no separate value…
        if arg == "--config" {
            // …except the space-separated form.
            args.next();
            continue;
        }
        if arg.starts_with("--") {
            continue;
        }
        return Some(arg);
    }
    None
}

/// Print an informational message to stdout for human-readable commands.
/// Suppressed entirely for subcommands that require clean stdout
/// (wallet commands and `get-block-data`).
pub fn print_info(msg: &str) {
    if !is_quiet_invocation() {
        println!("{}", msg);
    }
}

#[cfg(test)]
mod tests {
    use super::is_quiet_command;

    fn quiet(args: &[&str]) -> bool {
        is_quiet_command(args.iter().map(|a| a.to_string()))
    }

    #[test]
    fn quiet_commands_are_detected_by_subcommand() {
        assert!(quiet(&["balance"]));
        assert!(quiet(&["--config", "demo/alice.toml", "sync"]));
        assert!(quiet(&["--config=demo/alice.toml", "sync"]));
        assert!(quiet(&["-c", "demo/alice.toml", "-v", "notes", "--all"]));
        assert!(quiet(&["-vc", "demo/alice.toml", "notes"]));
        assert!(quiet(&["-cdemo/alice.toml", "notes"]));

        assert!(!quiet(&["test-orchard-zsa"]));
        assert!(!quiet(&[]));
        assert!(!quiet(&["--verbose"]));
    }

    #[test]
    fn arguments_that_spell_a_command_do_not_silence_a_noisy_one() {
        // An asset named "mine" is an argument, not the subcommand.
        assert!(!quiet(&["test-orchard-zsa", "mine"]));
        assert!(!quiet(&["--config", "sync", "test-orchard"]));
        // …and the real subcommand still decides when it is a quiet one.
        assert!(quiet(&["issue", "mine", "1000"]));
    }
}
