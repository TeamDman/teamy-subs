use crate::cli::output::CliOutput;
use arbitrary::Arbitrary;
use eyre::Context;
use eyre::Result;
use facet::Facet;

/// Delete the cache files.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct CacheCleanArgs;

#[derive(Facet, Debug)]
struct CacheCleanReport {
    entries_removed: usize,
}

impl CacheCleanArgs {
    /// # Errors
    ///
    /// This function will return an error if deleting cache entries fails.
    #[expect(clippy::unused_async)]
    pub async fn invoke(self) -> Result<CliOutput> {
        let result = crate::paths::clean_cache().wrap_err("Failed to clean cache directory")?;
        Ok(CliOutput::facet(CacheCleanReport {
            entries_removed: result.entries_removed,
        }))
    }
}
