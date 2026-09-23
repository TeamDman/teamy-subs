use crate::cli::output::CliOutput;
use crate::cli::subdl::account::show::SubdlAccountShowArgs;
use crate::cli::subdl::api::SubdlClient;
use arbitrary::Arbitrary;
use eyre::Result;
use facet::Facet;
use figue as args;
use teamy_cancellation::CancellationToken;

#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct SubdlAccountArgs {
    #[facet(args::subcommand)]
    pub command: SubdlAccountCommand,
}

#[derive(Facet, Arbitrary, Debug, PartialEq)]
#[repr(u8)]
pub enum SubdlAccountCommand {
    /// Show plan and usage.
    Show(SubdlAccountShowArgs),
}

impl SubdlAccountArgs {
    /// # Errors
    /// Returns an error if the command fails.
    pub async fn invoke(
        self,
        client: &SubdlClient,
        cancellation: &CancellationToken,
    ) -> Result<CliOutput> {
        match self.command {
            SubdlAccountCommand::Show(args) => args.invoke(client, cancellation).await,
        }
    }
}
