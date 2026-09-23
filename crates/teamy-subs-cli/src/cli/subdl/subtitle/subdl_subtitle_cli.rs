use crate::cli::output::CliOutput;
use crate::cli::subdl::api::SubdlClient;
use crate::cli::subdl::subtitle::download::SubdlSubtitleDownloadArgs;
use crate::cli::subdl::subtitle::search::SubdlSubtitleSearchArgs;
use arbitrary::Arbitrary;
use eyre::Result;
use facet::Facet;
use figue as args;
use teamy_cancellation::CancellationToken;

#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct SubdlSubtitleArgs {
    #[facet(args::subcommand)]
    pub command: SubdlSubtitleCommand,
}

#[derive(Facet, Arbitrary, Debug, PartialEq)]
#[repr(u8)]
pub enum SubdlSubtitleCommand {
    /// Download a subtitle for a local video file.
    Download(SubdlSubtitleDownloadArgs),
    /// Show ranked subtitle matches for a local video file.
    Search(SubdlSubtitleSearchArgs),
}

impl SubdlSubtitleArgs {
    /// # Errors
    /// Returns an error if the command fails.
    pub async fn invoke(
        self,
        client: &SubdlClient,
        cancellation: &CancellationToken,
    ) -> Result<CliOutput> {
        match self.command {
            SubdlSubtitleCommand::Download(args) => args.invoke(client, cancellation).await,
            SubdlSubtitleCommand::Search(args) => args.invoke(client, cancellation).await,
        }
    }
}
