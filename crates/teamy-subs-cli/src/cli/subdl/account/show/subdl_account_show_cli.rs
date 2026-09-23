use crate::cli::output::CliOutput;
use crate::cli::subdl::api::SubdlClient;
use arbitrary::Arbitrary;
use eyre::Result;
use facet::Facet;
use teamy_cancellation::CancellationToken;

#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct SubdlAccountShowArgs;

#[derive(Facet, Debug)]
struct AccountReport {
    plan: String,
    searches_used: u64,
    searches_limit: u64,
    searches_remaining: u64,
    downloads_used: u64,
    downloads_limit: u64,
    downloads_remaining: u64,
}

impl SubdlAccountShowArgs {
    /// # Errors
    /// Returns an error if the SubDL account request fails.
    pub async fn invoke(
        self,
        client: &SubdlClient,
        cancellation: &CancellationToken,
    ) -> Result<CliOutput> {
        let account = client.account(cancellation).await?;
        Ok(CliOutput::facet(AccountReport {
            plan: account.plan.name,
            searches_used: account.usage.search.used,
            searches_limit: account.usage.search.limit,
            searches_remaining: account.usage.search.remaining,
            downloads_used: account.usage.downloads.used,
            downloads_limit: account.usage.downloads.limit,
            downloads_remaining: account.usage.downloads.remaining,
        }))
    }
}
