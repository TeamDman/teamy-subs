use crate::cli::output::CliOutput;
use arbitrary::Arbitrary;
use eyre::Context;
use eyre::Result;
use facet::Facet;
use figue::{self as args};
use std::path::PathBuf;
use teamy_cancellation::CancellationToken;
use tokio::process::Command;
use tokio::time::Duration;
use tokio::time::sleep;

/// Download subtitles for remote media with `yt-dlp`.
// r[impl cli.command.yt-dlp.download]
#[derive(Facet, Arbitrary, Debug, PartialEq)]
#[facet(rename_all = "kebab-case")]
pub struct YtDlpDownloadArgs {
    /// Media URL to inspect with `yt-dlp`.
    #[facet(args::positional)]
    pub url: String,

    /// Directory where subtitle files should be written.
    #[facet(args::named)]
    pub output_dir: Option<String>,

    /// Language selector passed through to `yt-dlp --sub-langs`.
    #[facet(args::named)]
    pub sub_langs: Option<String>,

    /// Skip auto-generated subtitles.
    #[facet(args::named, default)]
    pub no_auto_subs: bool,
}

#[derive(Facet, Debug)]
struct YtDlpDownloadReport {
    output_dir: String,
}

impl YtDlpDownloadArgs {
    /// # Errors
    ///
    /// This function will return an error if the output directory cannot be created,
    /// `yt-dlp` cannot be started, or the process exits unsuccessfully.
    pub async fn invoke(self, cancellation_token: CancellationToken) -> Result<CliOutput> {
        cancellation_token.bail_if_cancelled()?;
        let output_dir = match self.output_dir {
            Some(path) => PathBuf::from(path),
            None => std::env::current_dir().wrap_err("Failed to determine current directory")?,
        };
        std::fs::create_dir_all(&output_dir).wrap_err_with(|| {
            format!("Failed to create output directory {}", output_dir.display())
        })?;

        let subtitle_languages = self.sub_langs.unwrap_or_else(|| "all".to_string());
        let mut command = Command::new("yt-dlp");
        command.kill_on_drop(true);
        command
            .arg("--skip-download")
            .arg("--write-subs")
            .arg("--sub-langs")
            .arg(&subtitle_languages)
            .arg("--sub-format")
            .arg("vtt")
            .arg("--convert-subs")
            .arg("vtt")
            .arg("--paths")
            .arg(format!("home:{}", output_dir.display()));

        if !self.no_auto_subs {
            command.arg("--write-auto-subs");
        }

        let mut child = command
            .arg(&self.url)
            .spawn()
            .wrap_err("Failed to launch yt-dlp. Ensure it is installed and available on PATH.")?;

        let status = loop {
            if cancellation_token.is_cancelled() {
                let _ = child.kill().await;
                cancellation_token.bail_if_cancelled()?;
            }
            if let Some(status) = child.try_wait().wrap_err("Failed to wait for yt-dlp")? {
                break status;
            }
            sleep(Duration::from_millis(100)).await;
        };

        if !status.success() {
            eyre::bail!("yt-dlp exited unsuccessfully with status {status}");
        }

        Ok(CliOutput::facet(YtDlpDownloadReport {
            output_dir: output_dir.display().to_string(),
        }))
    }
}
