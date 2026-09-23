use crate::cli::output::CliOutput;
use crate::cli::subdl::api::read_op_key;
use crate::cli::subdl::auth;
use arbitrary::Arbitrary;
use chrono::DateTime;
use chrono::Utc;
use eyre::Result;
use eyre::eyre;
use facet::Facet;
use figue as args;
use teamy_cancellation::CancellationToken;

/// Read a SubDL key from 1Password once and cache it for a bounded session.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct SubdlLoginArgs {
    /// Lease lifetime in whole minutes (1 to 60, default 15).
    #[facet(args::named)]
    pub ttl_minutes: Option<u32>,
}

#[derive(Facet)]
struct LoginReport {
    expires_at: String,
    ttl_minutes: u32,
}

impl SubdlLoginArgs {
    /// # Errors
    /// Returns an error if 1Password or secure lease creation fails.
    pub async fn invoke(
        self,
        op_ref: Option<&str>,
        cancellation: &CancellationToken,
    ) -> Result<CliOutput> {
        let ttl_minutes = self.ttl_minutes.unwrap_or(15);
        if !(1..=60).contains(&ttl_minutes) {
            eyre::bail!("--ttl-minutes must be between 1 and 60");
        }
        let reference = op_ref
            .map(str::to_owned)
            .or_else(|| std::env::var("SUBDL_API_KEY_OP_REF").ok())
            .ok_or_else(|| {
                eyre!("Pass --op-ref to subdl or set SUBDL_API_KEY_OP_REF before login")
            })?;
        let key = read_op_key(&reference, cancellation).await?;
        cancellation.bail_if_cancelled()?;
        let expires_at = auth::store_key(key, ttl_minutes)?;
        let expires_at = DateTime::<Utc>::from_timestamp(expires_at, 0)
            .ok_or_else(|| eyre!("Invalid SubDL lease expiry"))?
            .to_rfc3339();
        Ok(CliOutput::facet(LoginReport {
            expires_at,
            ttl_minutes,
        }))
    }
}
