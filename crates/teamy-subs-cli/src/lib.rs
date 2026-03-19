#![deny(clippy::disallowed_methods)]
#![deny(clippy::disallowed_macros)]

pub mod cli;
pub mod logging_init;
pub mod paths;

use crate::cli::Cli;

/// Run the teamy-subs CLI using externally supplied version metadata.
///
/// # Errors
///
/// This function will return an error if setup, CLI parsing, logging initialization,
/// or command execution fails.
///
/// # Panics
///
/// Panics if the CLI schema is invalid (should never happen with correct code).
pub fn run(version: &str, implementation_git_repo: &str, git_revision: &str) -> eyre::Result<()> {
    color_eyre::install()?;

    let cli: Cli = figue::Driver::new(
        figue::builder::<Cli>()
            .expect("schema should be valid")
            .cli(move |cli| cli.args_os(std::env::args_os().skip(1)).strict())
            .help(move |help| {
                help.version(version)
                    .include_implementation_source_file(true)
                    .include_implementation_git_url(implementation_git_repo, git_revision)
            })
            .build(),
    )
    .run()
    .unwrap();

    logging_init::init_logging(&cli.global_args)?;

    #[cfg(windows)]
    {
        let _ = teamy_windows::console::enable_ansi_support();
        teamy_windows::string::warn_if_utf8_not_enabled();
    };

    cli.invoke()?;
    Ok(())
}
