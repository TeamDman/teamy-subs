use crate::cli::output::CliOutput;
use crate::cli::subdl::auth;
use arbitrary::Arbitrary;
use eyre::Result;
use facet::Facet;

/// Show cached SubDL login state without reading 1Password.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct SubdlStatusArgs;

#[derive(Facet)]
struct StatusReport {
    active_leases: usize,
    maximum_remaining_seconds: i64,
}

impl SubdlStatusArgs {
    /// # Errors
    /// Returns an error if local lease metadata cannot be read.
    pub fn invoke(self) -> Result<CliOutput> {
        let (active_leases, maximum_remaining_seconds) = auth::status()?;
        Ok(CliOutput::facet(StatusReport {
            active_leases,
            maximum_remaining_seconds,
        }))
    }
}
