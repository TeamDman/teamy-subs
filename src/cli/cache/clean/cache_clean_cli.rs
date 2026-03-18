use arbitrary::Arbitrary;
use eyre::Context;
use eyre::Result;
use facet::Facet;

/// Delete the cache files.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct CacheCleanArgs;

impl CacheCleanArgs {
    /// # Errors
    ///
    /// This function will return an error if deleting cache entries fails.
    #[expect(clippy::unused_async)]
    pub async fn invoke(self) -> Result<()> {
        let result = crate::paths::clean_cache().wrap_err("Failed to clean cache directory")?;
        println!("Removed {} cache entrie(s).", result.entries_removed);
        Ok(())
    }
}
