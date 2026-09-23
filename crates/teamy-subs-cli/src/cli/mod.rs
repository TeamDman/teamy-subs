pub mod cache;
pub mod convert;
pub mod extract;
pub mod facet_shape;
pub mod global_args;
pub mod home;
pub mod output;
pub mod subdl;
pub mod yt_dlp;

use crate::cli::cache::CacheArgs;
use crate::cli::convert::ConvertArgs;
use crate::cli::extract::ExtractArgs;
use crate::cli::global_args::GlobalArgs;
use crate::cli::home::HomeArgs;
use crate::cli::output::CliOutput;
use crate::cli::subdl::SubdlArgs;
use crate::cli::yt_dlp::YtDlpArgs;
use arbitrary::Arbitrary;
use eyre::Context;
use facet::Facet;
use figue::FigueBuiltins;
use figue::{self as args};
use teamy_cancellation::CancellationToken;

/// Command line utility for downloading, extracting, and converting subtitles.
#[derive(Facet, Arbitrary, Debug)]
pub struct Cli {
    /// Global arguments (`debug`, `log_filter`, `log_file`).
    #[facet(flatten)]
    pub global_args: GlobalArgs,

    /// Standard CLI options (help, version, completions).
    #[facet(flatten)]
    #[arbitrary(default)]
    pub builtins: FigueBuiltins,

    /// The command to run.
    #[facet(args::subcommand)]
    pub command: Command,
}

impl PartialEq for Cli {
    fn eq(&self, other: &Self) -> bool {
        // Ignore builtins in comparison since FigueBuiltins doesn't implement PartialEq
        self.global_args == other.global_args && self.command == other.command
    }
}

impl Cli {
    /// # Errors
    ///
    /// This function will return an error if the tokio runtime cannot be built or if the command fails.
    pub fn invoke(self, cancellation_token: CancellationToken) -> eyre::Result<CliOutput> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .wrap_err("Failed to build tokio runtime")?;
        runtime.block_on(async move { self.command.invoke(cancellation_token).await })
    }
}

/// Supported top-level commands.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
#[repr(u8)]
pub enum Command {
    /// Cache-related commands.
    Cache(CacheArgs),
    // r[impl cli.command.convert]
    /// Convert subtitle files into other formats.
    Convert(ConvertArgs),
    // r[impl cli.command.extract]
    /// Extract subtitle streams from local media with `ffmpeg`.
    Extract(ExtractArgs),
    /// Home-related commands.
    Home(HomeArgs),
    /// Commands backed by SubDL.
    Subdl(SubdlArgs),
    /// Commands backed by yt-dlp.
    YtDlp(YtDlpArgs),
}

impl Command {
    /// # Errors
    ///
    /// This function will return an error if the subcommand fails.
    pub async fn invoke(self, cancellation_token: CancellationToken) -> eyre::Result<CliOutput> {
        cancellation_token.bail_if_cancelled()?;
        match self {
            Command::Cache(args) => args.invoke().await,
            Command::Convert(args) => args.invoke(&cancellation_token).await,
            Command::Extract(args) => args.invoke(&cancellation_token).await,
            Command::Home(args) => args.invoke().await,
            Command::Subdl(args) => args.invoke(cancellation_token).await,
            Command::YtDlp(args) => args.invoke(cancellation_token).await,
        }
    }
}
