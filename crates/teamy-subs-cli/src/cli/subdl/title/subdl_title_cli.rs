use crate::cli::output::CliOutput;
use crate::cli::subdl::api::SubdlClient;
use crate::cli::subdl::title::search::SubdlTitleSearchArgs;
use arbitrary::Arbitrary;
use eyre::Result;
use facet::Facet;
use figue as args;
use teamy_cancellation::CancellationToken;

#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct SubdlTitleArgs {
    #[facet(args::subcommand)]
    pub command: SubdlTitleCommand,
}

#[derive(Facet, Arbitrary, Debug, PartialEq)]
#[repr(u8)]
pub enum SubdlTitleCommand {
    /// Search movie and TV titles.
    Search(SubdlTitleSearchArgs),
}

impl SubdlTitleArgs {
    /// # Errors
    /// Returns an error if the command fails.
    pub async fn invoke(
        self,
        client: &SubdlClient,
        cancellation: &CancellationToken,
    ) -> Result<CliOutput> {
        match self.command {
            SubdlTitleCommand::Search(args) => args.invoke(client, cancellation).await,
        }
    }
}
