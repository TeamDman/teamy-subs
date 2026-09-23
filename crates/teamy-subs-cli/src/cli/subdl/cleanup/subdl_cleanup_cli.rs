use crate::cli::output::CliOutput;
use crate::cli::subdl::auth;
use arbitrary::Arbitrary;
use eyre::Result;
use facet::Facet;
use figue as args;

/// Internal scheduled cleanup for one generated lease.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct SubdlCleanupArgs {
    /// Generated lease identity; arbitrary paths are rejected.
    #[facet(args::named)]
    pub lease: String,
}

impl SubdlCleanupArgs {
    /// # Errors
    /// Returns an error if the lease identity is invalid or cleanup fails.
    pub fn invoke(self) -> Result<CliOutput> {
        auth::cleanup(&self.lease)?;
        Ok(CliOutput::none())
    }
}
