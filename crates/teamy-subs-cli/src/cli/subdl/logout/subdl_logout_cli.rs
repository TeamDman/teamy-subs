use crate::cli::output::CliOutput;
use crate::cli::subdl::auth;
use arbitrary::Arbitrary;
use eyre::Result;
use facet::Facet;

/// Remove all cached SubDL login leases now.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct SubdlLogoutArgs;

#[derive(Facet)]
struct LogoutReport {
    removed_leases: usize,
}

impl SubdlLogoutArgs {
    /// # Errors
    /// Returns an error if a lease or its cleanup task cannot be removed.
    pub fn invoke(self) -> Result<CliOutput> {
        Ok(CliOutput::facet(LogoutReport {
            removed_leases: auth::logout()?,
        }))
    }
}
