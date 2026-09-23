//! Bounded, encrypted SubDL login leases shared by separate CLI processes.

#[cfg(windows)]
use super::auth_windows as native;
use chrono::Utc;
use eyre::Context;
use eyre::Result;
use eyre::bail;
use eyre::eyre;
use facet::Facet;
use std::path::Path;
use std::path::PathBuf;

const LEASE_SCHEMA: &str = "teamy-subs.subdl-lease/1";
const LEASE_PURPOSE: &str = "subdl-api";
const MAX_TTL_MINUTES: u32 = 60;
pub const LOGIN_HINT: &str =
    "Run teamy-subs subdl --op-ref <reference> login, or set SUBDL_API_KEY";

// Deliberately no Debug: parse errors must not include decrypted credentials.
#[derive(Facet)]
struct Lease {
    schema: String,
    purpose: String,
    issued_at: i64,
    expires_at: i64,
    #[facet(sensitive)]
    api_key: String,
}

impl Lease {
    fn valid(&self, now: i64) -> bool {
        self.schema == LEASE_SCHEMA
            && self.purpose == LEASE_PURPOSE
            && self
                .expires_at
                .checked_sub(self.issued_at)
                .is_some_and(|ttl| (1..=i64::from(MAX_TTL_MINUTES) * 60).contains(&ttl))
            && now >= self.issued_at
            && now < self.expires_at
            && !self.api_key.trim().is_empty()
    }
}

pub(super) fn auth_root() -> Result<PathBuf> {
    if !cfg!(windows) {
        bail!("Cached SubDL login currently requires Windows");
    }
    let dirs = directories_next::ProjectDirs::from("", "teamdman", "teamy-subs")
        .ok_or_else(|| eyre!("Cannot determine private credential storage directory"))?;
    Ok(dirs.data_local_dir().join("credentials").join("subdl"))
}

pub(super) fn valid_lease_name(name: &str) -> bool {
    name.strip_prefix("lease-")
        .and_then(|value| value.strip_suffix(".bin"))
        .is_some_and(|id| id.len() == 32 && id.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

fn lease_paths(root: &Path) -> Result<Vec<PathBuf>> {
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry?;
        if entry.file_name().to_str().is_some_and(valid_lease_name) {
            paths.push(entry.path());
        }
    }
    paths.sort();
    Ok(paths)
}

fn read_lease(path: &Path) -> Result<Lease> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| eyre!("Invalid SubDL lease filename"))?;
    let root = path
        .parent()
        .ok_or_else(|| eyre!("Missing SubDL lease directory"))?;
    let bytes = native::read(root, name)?;
    let text = String::from_utf8(bytes).map_err(|_| eyre!("Invalid SubDL lease encoding"))?;
    facet_json::from_str(&text).map_err(|_| eyre!("Invalid SubDL lease payload"))
}

/// Read a current lease without contacting 1Password.
pub fn load_key() -> Result<String> {
    let root = auth_root()?;
    let now = Utc::now().timestamp();
    let mut selected: Option<Lease> = None;
    for path in lease_paths(&root)? {
        match read_lease(&path) {
            Ok(lease) if lease.valid(now) => {
                if selected
                    .as_ref()
                    .is_none_or(|previous| previous.issued_at < lease.issued_at)
                {
                    selected = Some(lease);
                }
            }
            _ => {
                if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
                    let _ = native::remove(&root, name);
                }
            }
        }
    }
    selected
        .map(|lease| lease.api_key)
        .ok_or_else(|| eyre!("No valid SubDL login. {LOGIN_HINT}"))
}

/// Store a key for a short, explicit lifetime with scheduled cleanup.
pub fn store_key(key: String, ttl_minutes: u32) -> Result<i64> {
    if !(1..=MAX_TTL_MINUTES).contains(&ttl_minutes) {
        bail!("--ttl-minutes must be between 1 and {MAX_TTL_MINUTES}");
    }
    if key.trim().is_empty() {
        bail!("Cannot store an empty SubDL API key");
    }
    let root = auth_root()?;
    native::prepare(&root)?;
    let now = Utc::now();
    let expiry = now + chrono::Duration::minutes(i64::from(ttl_minutes));
    let lease = Lease {
        schema: LEASE_SCHEMA.to_owned(),
        purpose: LEASE_PURPOSE.to_owned(),
        issued_at: now.timestamp(),
        expires_at: expiry.timestamp(),
        api_key: key,
    };
    let payload =
        facet_json::to_string(&lease).map_err(|_| eyre!("Cannot encode SubDL login lease"))?;
    let name = format!("lease-{:032x}.bin", rand::random::<u128>());
    native::write(
        &root,
        &name,
        expiry,
        payload.as_bytes(),
        &std::env::current_exe().wrap_err("Cannot locate CLI executable for cleanup")?,
    )?;
    if let Ok(paths) = lease_paths(&root) {
        for path in paths {
            let Some(previous) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if previous != name {
                let _ = native::remove(&root, previous);
            }
        }
    }
    Ok(expiry.timestamp())
}

/// Report lease state without printing or returning any credential.
pub fn status() -> Result<(usize, i64)> {
    let root = auth_root()?;
    let now = Utc::now().timestamp();
    let mut active = 0;
    let mut latest = now;
    for path in lease_paths(&root)? {
        match read_lease(&path) {
            Ok(lease) if lease.valid(now) => {
                active += 1;
                latest = latest.max(lease.expires_at);
            }
            _ => {
                if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
                    let _ = native::remove(&root, name);
                }
            }
        }
    }
    Ok((active, latest - now))
}

/// Remove all SubDL leases and their own cleanup tasks.
pub fn logout() -> Result<usize> {
    let root = auth_root()?;
    let paths = lease_paths(&root)?;
    for path in &paths {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| eyre!("Invalid SubDL lease filename"))?;
        native::remove(&root, name)?;
    }
    Ok(paths.len())
}

/// Remove one generated lease, for the scheduled cleanup action.
pub fn cleanup(name: &str) -> Result<()> {
    if !valid_lease_name(name) {
        bail!("Invalid SubDL lease identity");
    }
    native::remove(&auth_root()?, name)
}

#[cfg(not(windows))]
mod native {
    use std::path::Path;

    pub(super) fn prepare(_: &Path) -> eyre::Result<()> {
        eyre::bail!("Cached SubDL login requires Windows")
    }
    pub(super) fn read(_: &Path, _: &str) -> eyre::Result<Vec<u8>> {
        eyre::bail!("Cached SubDL login requires Windows")
    }
    pub(super) fn write(
        _: &Path,
        _: &str,
        _: chrono::DateTime<chrono::Utc>,
        _: &[u8],
        _: &Path,
    ) -> eyre::Result<()> {
        eyre::bail!("Cached SubDL login requires Windows")
    }
    pub(super) fn remove(_: &Path, _: &str) -> eyre::Result<()> {
        eyre::bail!("Cached SubDL login requires Windows")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lease_expires_at_boundary_and_rejects_other_purposes() {
        let mut lease = Lease {
            schema: LEASE_SCHEMA.to_owned(),
            purpose: LEASE_PURPOSE.to_owned(),
            issued_at: 100,
            expires_at: 200,
            api_key: "dummy-only".to_owned(),
        };
        assert!(lease.valid(100));
        assert!(lease.valid(199));
        assert!(!lease.valid(200));
        assert!(!lease.valid(99));
        lease.purpose = "other".to_owned();
        assert!(!lease.valid(150));
    }

    #[test]
    fn cleanup_only_accepts_generated_names() {
        assert!(valid_lease_name(
            "lease-00000000000000000000000000000001.bin"
        ));
        for invalid in ["../lease-123.bin", "lease-*.bin", "user-file.txt"] {
            assert!(!valid_lease_name(invalid));
        }
    }
}
