#![deny(clippy::disallowed_methods)]
#![deny(clippy::disallowed_macros)]

pub use teamy_subs_cli::{cli, logging_init, paths};

/// Version string combining package version and git revision.
const VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (rev ",
    env!("GIT_REVISION"),
    ")"
);
const GIT_REVISION: &str = env!("GIT_REVISION");

/// Entrypoint for the program.
///
/// # Errors
///
/// This function will return an error if `color_eyre` installation, CLI parsing, logging initialization, or command execution fails.
///
/// # Panics
///
/// Panics if the CLI schema is invalid (should never happen with correct code).
pub fn main() -> eyre::Result<()> {
    teamy_subs_cli::run(VERSION, "TeamDman/teamy-subs", GIT_REVISION)
}
