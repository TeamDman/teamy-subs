//! Private SubDL transport and response types. Download URLs can contain credentials.

use crate::cli::subdl::auth;
use eyre::Result;
use eyre::bail;
use eyre::eyre;
use facet::Facet;
use reqwest::Client;
use reqwest::Url;
use std::future::Future;
use std::process::Stdio;
use std::time::Duration;
use teamy_cancellation::CancellationToken;
use tokio::process::Command;

const MAX_JSON_BYTES: usize = 2 * 1024 * 1024;
const MAX_ARCHIVE_BYTES: usize = 20 * 1024 * 1024;

pub struct SubdlClient {
    http: Client,
    api_base: Url,
    download_base: Url,
    key: String,
}

#[derive(Facet)]
pub struct FileSearchResponse {
    pub status: bool,
    #[facet(default)]
    pub results: Vec<Title>,
    #[facet(rename = "match", default)]
    pub title_match: Option<TitleMatch>,
    #[facet(default)]
    pub subtitles: Vec<SubtitleCandidate>,
}

#[derive(Facet)]
pub struct SubtitleCandidate {
    #[facet(default)]
    pub name: String,
    #[facet(default)]
    pub release_name: String,
    #[facet(default)]
    pub lang: String,
    #[facet(default)]
    pub language: Option<String>,
    #[facet(default)]
    pub match_score: Option<f64>,
    #[facet(default)]
    pub url: String,
}

#[derive(Facet)]
pub struct TitleMatch {
    pub title: String,
    #[facet(default)]
    pub year: Option<u16>,
    #[facet(default)]
    pub confidence: Option<String>,
    #[facet(default)]
    pub engine: Option<String>,
}

#[derive(Facet)]
pub struct Title {
    pub name: String,
    #[facet(default)]
    pub year: Option<u16>,
    #[facet(default)]
    pub r#type: Option<String>,
    #[facet(default)]
    pub imdb_id: Option<String>,
    #[facet(default)]
    pub tmdb_id: Option<u64>,
}

#[derive(Facet)]
pub struct TitleSearchResponse {
    #[facet(default)]
    pub results: Vec<Title>,
}

#[derive(Facet)]
pub struct AccountResponse {
    pub plan: Plan,
    pub usage: Usage,
}

#[derive(Facet)]
pub struct Plan {
    pub name: String,
}

#[derive(Facet)]
pub struct Usage {
    pub search: Quota,
    pub downloads: Quota,
}

#[derive(Facet)]
pub struct Quota {
    pub used: u64,
    pub limit: u64,
    pub remaining: u64,
}

impl SubdlClient {
    /// # Errors
    /// Returns an error if the key cannot be acquired or the HTTP client cannot be built.
    pub async fn new(op_ref: Option<&str>, cancellation: &CancellationToken) -> Result<Self> {
        let key = read_key(op_ref, cancellation).await?;
        let http = Client::builder()
            .timeout(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| eyre!("Failed to create SubDL HTTP client"))?;
        Ok(Self {
            http,
            api_base: Url::parse("https://api.subdl.com/")?,
            download_base: Url::parse("https://dl.subdl.com/")?,
            key,
        })
    }

    /// # Errors
    /// Returns an error if the search request or response is invalid.
    pub async fn search_file(
        &self,
        filename: &str,
        language: &str,
        cancellation: &CancellationToken,
    ) -> Result<FileSearchResponse> {
        self.get_json(
            "api/v2/files/search",
            &[
                ("filename", filename),
                ("languages", language),
                ("subs_per_page", "30"),
            ],
            cancellation,
        )
        .await
    }

    /// # Errors
    /// Returns an error if the title search request or response is invalid.
    pub async fn search_title(
        &self,
        query: &str,
        cancellation: &CancellationToken,
    ) -> Result<TitleSearchResponse> {
        self.get_json("api/v2/movies/search", &[("q", query)], cancellation)
            .await
    }

    /// # Errors
    /// Returns an error if the account request or response is invalid.
    pub async fn account(&self, cancellation: &CancellationToken) -> Result<AccountResponse> {
        self.get_json("api/v2/me", &[], cancellation).await
    }

    async fn get_json<T: Facet<'static>>(
        &self,
        endpoint: &str,
        params: &[(&str, &str)],
        cancellation: &CancellationToken,
    ) -> Result<T> {
        cancellation.bail_if_cancelled()?;
        let mut url = self.api_base.join(endpoint)?;
        url.query_pairs_mut().extend_pairs(params.iter().copied());
        let response = cancellable(
            cancellation,
            self.http.get(url).bearer_auth(&self.key).send(),
        )
        .await?
        .map_err(|_| eyre!("SubDL request failed"))?;
        if !response.status().is_success() {
            bail!("SubDL returned HTTP {}", response.status().as_u16());
        }
        let bytes = read_limited(response, MAX_JSON_BYTES, cancellation).await?;
        let body =
            std::str::from_utf8(&bytes).map_err(|_| eyre!("SubDL returned invalid UTF-8"))?;
        facet_json::from_str(body).map_err(|_| eyre!("SubDL returned an unexpected JSON shape"))
    }

    /// Download using only the path from SubDL's URL, with the key in a header.
    /// # Errors
    /// Returns an error if the URL or response is invalid.
    pub async fn download_archive(
        &self,
        raw_url: &str,
        cancellation: &CancellationToken,
    ) -> Result<Vec<u8>> {
        let path = raw_url.split('?').next().unwrap_or_default();
        if !path.starts_with("/subtitle/")
            || path.contains("..")
            || path.contains(':')
            || !path.ends_with(".zip")
        {
            bail!("SubDL returned an unexpected download path");
        }
        let url = self.download_base.join(path.trim_start_matches('/'))?;
        let response = cancellable(
            cancellation,
            self.http.get(url).header("X-API-Key", &self.key).send(),
        )
        .await?
        .map_err(|_| eyre!("SubDL download request failed"))?;
        if !response.status().is_success() {
            bail!(
                "SubDL download returned HTTP {}",
                response.status().as_u16()
            );
        }
        read_limited(response, MAX_ARCHIVE_BYTES, cancellation).await
    }
}

async fn read_key(op_ref: Option<&str>, cancellation: &CancellationToken) -> Result<String> {
    if let Some(reference) = op_ref {
        return read_op_key(reference, cancellation).await;
    }
    if let Ok(key) = std::env::var("SUBDL_API_KEY") {
        if key.is_empty() {
            bail!("SUBDL_API_KEY is empty");
        }
        return Ok(key);
    }
    auth::load_key()
}

pub(super) async fn read_op_key(
    reference: &str,
    cancellation: &CancellationToken,
) -> Result<String> {
    let mut command = Command::new("op");
    command
        .args(["read", reference, "--no-newline"])
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    let output = cancellable(cancellation, command.output())
        .await?
        .map_err(|_| eyre!("Unable to start 1Password CLI; put op on PATH"))?;
    if !output.status.success() {
        bail!("1Password could not read the SubDL API key");
    }
    let key = String::from_utf8(output.stdout)
        .map_err(|_| eyre!("1Password returned an invalid SubDL API key"))?;
    if key.is_empty() {
        bail!("1Password returned an empty SubDL API key");
    }
    Ok(key)
}

async fn cancellable<F: Future>(cancellation: &CancellationToken, future: F) -> Result<F::Output> {
    tokio::pin!(future);
    loop {
        tokio::select! {
            value = &mut future => return Ok(value),
            () = tokio::time::sleep(Duration::from_millis(100)) => cancellation.bail_if_cancelled()?,
        }
    }
}

async fn read_limited(
    mut response: reqwest::Response,
    max_bytes: usize,
    cancellation: &CancellationToken,
) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    while let Some(chunk) = cancellable(cancellation, response.chunk())
        .await?
        .map_err(|_| eyre!("Failed to read SubDL response"))?
    {
        if chunk.len() > max_bytes.saturating_sub(bytes.len()) {
            bail!("SubDL response exceeded size limit");
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncReadExt;
    use tokio::io::AsyncWriteExt;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn download_uses_header_and_discards_key_bearing_query() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            loop {
                let mut chunk = [0_u8; 1024];
                let count = socket.read(&mut chunk).await.unwrap();
                if count == 0 {
                    break;
                }
                request.extend_from_slice(&chunk[..count]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\nTEST")
                .await
                .unwrap();
            String::from_utf8(request).unwrap()
        });
        let client = SubdlClient {
            http: Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .unwrap(),
            api_base: Url::parse("http://127.0.0.1/").unwrap(),
            download_base: Url::parse(&format!("http://{address}/")).unwrap(),
            key: "dummy-secret".to_string(),
        };
        let bytes = client
            .download_archive(
                "/subtitle/1-2.zip?api_key=dummy-secret",
                &CancellationToken::new(),
            )
            .await
            .unwrap();
        assert_eq!(bytes, b"TEST");
        let request = server.await.unwrap();
        assert!(request.starts_with("GET /subtitle/1-2.zip HTTP/1.1\r\n"));
        assert!(
            request
                .to_ascii_lowercase()
                .contains("x-api-key: dummy-secret")
        );
        assert!(!request.contains("api_key="));
    }
}
