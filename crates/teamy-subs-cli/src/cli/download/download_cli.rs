use arbitrary::Arbitrary;
use eyre::Context;
use eyre::Result;
use eyre::bail;
use facet::Facet;
use figue::{self as args};
use std::path::PathBuf;
use std::process::Command;

/// Download subtitles for remote media with `yt-dlp`.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
#[facet(rename_all = "kebab-case")]
pub struct DownloadArgs {
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

impl DownloadArgs {
    /// # Errors
    ///
    /// This function will return an error if the output directory cannot be created,
    /// `yt-dlp` cannot be started, or the process exits unsuccessfully.
    #[expect(clippy::unused_async)]
    pub async fn invoke(self) -> Result<()> {
        let output_dir = match self.output_dir {
            Some(path) => PathBuf::from(path),
            None => std::env::current_dir().wrap_err("Failed to determine current directory")?,
        };
        std::fs::create_dir_all(&output_dir).wrap_err_with(|| {
            format!("Failed to create output directory {}", output_dir.display())
        })?;

        let subtitle_languages = self.sub_langs.unwrap_or_else(|| "all".to_string());
        let mut command = Command::new("yt-dlp");
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

        let status = command
            .arg(&self.url)
            .status()
            .wrap_err("Failed to launch yt-dlp. Ensure it is installed and available on PATH.")?;

        if !status.success() {
            bail!("yt-dlp exited unsuccessfully with status {status}");
        }

        println!("Downloaded subtitles into {}", output_dir.display());
        Ok(())
    }
}
