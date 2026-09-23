use crate::cli::output::CliOutput;
use crate::cli::subdl::account::SubdlAccountArgs;
use crate::cli::subdl::api::SubdlClient;
use crate::cli::subdl::cleanup::SubdlCleanupArgs;
use crate::cli::subdl::login::SubdlLoginArgs;
use crate::cli::subdl::logout::SubdlLogoutArgs;
use crate::cli::subdl::status::SubdlStatusArgs;
use crate::cli::subdl::subtitle::SubdlSubtitleArgs;
use crate::cli::subdl::title::SubdlTitleArgs;
use arbitrary::Arbitrary;
use eyre::Result;
use facet::Facet;
use figue as args;
use teamy_cancellation::CancellationToken;

/// Search and download subtitles from SubDL.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
#[facet(rename_all = "kebab-case")]
pub struct SubdlArgs {
    /// 1Password secret reference containing your SubDL API key.
    #[facet(args::named)]
    pub op_ref: Option<String>,

    #[facet(args::subcommand)]
    pub command: SubdlCommand,
}

#[derive(Facet, Arbitrary, Debug, PartialEq)]
#[repr(u8)]
pub enum SubdlCommand {
    /// Show your plan and remaining quota.
    Account(SubdlAccountArgs),
    /// Internal cleanup of an expired login lease.
    Cleanup(SubdlCleanupArgs),
    /// Cache a SubDL key from 1Password for a short session.
    Login(SubdlLoginArgs),
    /// Remove all cached SubDL login leases.
    Logout(SubdlLogoutArgs),
    /// Show remaining cached login time.
    Status(SubdlStatusArgs),
    /// Find and download subtitles.
    Subtitle(SubdlSubtitleArgs),
    /// Search movie and TV titles.
    Title(SubdlTitleArgs),
}

impl SubdlArgs {
    /// # Errors
    /// Returns an error if authentication or the selected command fails.
    pub async fn invoke(self, cancellation: CancellationToken) -> Result<CliOutput> {
        cancellation.bail_if_cancelled()?;
        match self.command {
            SubdlCommand::Cleanup(args) => args.invoke(),
            SubdlCommand::Login(args) => args.invoke(self.op_ref.as_deref(), &cancellation).await,
            SubdlCommand::Logout(args) => args.invoke(),
            SubdlCommand::Status(args) => args.invoke(),
            SubdlCommand::Account(args) => {
                let client = SubdlClient::new(self.op_ref.as_deref(), &cancellation).await?;
                args.invoke(&client, &cancellation).await
            }
            SubdlCommand::Subtitle(args) => {
                let client = SubdlClient::new(self.op_ref.as_deref(), &cancellation).await?;
                args.invoke(&client, &cancellation).await
            }
            SubdlCommand::Title(args) => {
                let client = SubdlClient::new(self.op_ref.as_deref(), &cancellation).await?;
                args.invoke(&client, &cancellation).await
            }
        }
    }
}
