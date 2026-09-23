#![deny(clippy::disallowed_methods)]
#![deny(clippy::disallowed_macros)]

pub mod cli;
pub mod logging_init;
pub mod paths;
#[cfg(windows)]
mod windows_startup;

use crate::cli::Cli;
use teamy_cancellation::CtrlCHandler;

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
pub fn run(version: String, implementation_git_repo: &str, git_revision: &str) -> eyre::Result<()> {
    color_eyre::install()?;
    let cancellation_token = CtrlCHandler::default().install()?;

    #[cfg(windows)]
    {
        let _ = windows_startup::enable_ansi_support();
    }

    let cli: Cli = figue::Driver::new(
        figue::builder::<Cli>()
            .expect("schema should be valid")
            .cli(move |cli| cli.args_os(std::env::args_os().skip(1)).strict())
            .help(move |help| {
                help.version(version)
                    .include_implementation_source_file(true)
                    .include_implementation_github_url(implementation_git_repo, git_revision)
            })
            .build(),
    )
    .run()
    .unwrap();

    let _stop_after_duration_thread = cli
        .global_args
        .stop_after
        .start_stop_after_duration_thread(cancellation_token.clone())?;

    logging_init::init_logging(&cli.global_args, cancellation_token.clone())?;

    #[cfg(windows)]
    {
        windows_startup::warn_if_utf8_not_enabled();
    };

    let output_format = cli.global_args.output_format;
    let output = cli.invoke(cancellation_token.clone())?;
    cancellation_token.bail_if_cancelled()?;
    output.emit(output_format)?;
    cancellation_token.bail_if_cancelled()?;
    Ok(())
}
