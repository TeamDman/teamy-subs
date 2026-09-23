use crate::cli::output::CliOutput;
use crate::cli::yt_dlp::download::YtDlpDownloadArgs;
use arbitrary::Arbitrary;
use eyre::Result;
use facet::Facet;
use figue as args;
use teamy_cancellation::CancellationToken;

/// Commands backed by yt-dlp.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct YtDlpArgs {
    #[facet(args::subcommand)]
    pub command: YtDlpCommand,
}

#[derive(Facet, Arbitrary, Debug, PartialEq)]
#[repr(u8)]
pub enum YtDlpCommand {
    /// Download subtitles for remote media.
    Download(YtDlpDownloadArgs),
}

impl YtDlpArgs {
    /// # Errors
    /// Returns an error if the selected command fails.
    pub async fn invoke(self, cancellation_token: CancellationToken) -> Result<CliOutput> {
        match self.command {
            YtDlpCommand::Download(args) => args.invoke(cancellation_token).await,
        }
    }
}
