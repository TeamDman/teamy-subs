use crate::cli::output::CliOutput;
use crate::cli::subdl::api::SubdlClient;
use crate::cli::subdl::api::SubtitleCandidate;
use arbitrary::Arbitrary;
use eyre::Context;
use eyre::Result;
use eyre::bail;
use eyre::eyre;
use facet::Facet;
use figue as args;
use std::fs::OpenOptions;
use std::io::Cursor;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use teamy_cancellation::CancellationToken;
use zip::ZipArchive;

const MAX_SUBTITLE_BYTES: u64 = 10 * 1024 * 1024;

#[derive(Facet, Arbitrary, Debug, PartialEq)]
#[facet(rename_all = "kebab-case")]
pub struct SubdlSubtitleDownloadArgs {
    /// Existing local video file to match by release filename.
    #[facet(args::named)]
    pub file: String,

    /// Subtitle language code, such as en or fr. Defaults to en.
    #[facet(args::named)]
    pub language: Option<String>,

    /// Directory in which to save the subtitle. Defaults to the current directory.
    #[facet(args::named)]
    pub output_dir: Option<String>,

    /// One-based candidate number from `subdl subtitle search`.
    #[facet(args::named)]
    pub pick: Option<usize>,

    /// Replace an existing subtitle file.
    #[facet(args::named, default)]
    pub force: bool,
}

#[derive(Facet, Debug)]
struct DownloadReport {
    path: String,
    release_name: String,
    language: String,
    match_score: Option<f64>,
}

impl SubdlSubtitleDownloadArgs {
    /// # Errors
    /// Returns an error if no confident match exists or the subtitle cannot be saved.
    pub async fn invoke(
        self,
        client: &SubdlClient,
        cancellation: &CancellationToken,
    ) -> Result<CliOutput> {
        cancellation.bail_if_cancelled()?;
        let video = Path::new(&self.file);
        if !video.is_file() {
            bail!("Video file does not exist: {}", video.display());
        }
        let filename = video
            .file_name()
            .and_then(|part| part.to_str())
            .ok_or_else(|| eyre!("Video file has no UTF-8 filename"))?;
        let stem = video
            .file_stem()
            .and_then(|part| part.to_str())
            .ok_or_else(|| eyre!("Video file has no UTF-8 stem"))?;
        let language = self
            .language
            .as_deref()
            .unwrap_or("en")
            .to_ascii_lowercase();
        if language.is_empty()
            || !language
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            bail!("--language must contain only letters, digits, or hyphens");
        }
        let response = client
            .search_file(filename, &language, cancellation)
            .await?;
        if !response.status {
            bail!("SubDL did not return a successful filename match");
        }
        let candidate = choose_candidate(&response.subtitles, self.pick)?;
        let release_name = candidate.release_name.clone();
        let match_score = candidate.match_score;
        let archive = client
            .download_archive(&candidate.url, cancellation)
            .await?;
        cancellation.bail_if_cancelled()?;
        let (extension, subtitle) = extract_subtitle(&archive)?;
        cancellation.bail_if_cancelled()?;

        let output_dir = self
            .output_dir
            .map_or_else(std::env::current_dir, |path| Ok(PathBuf::from(path)))
            .wrap_err("Failed to resolve output directory")?;
        std::fs::create_dir_all(&output_dir).wrap_err_with(|| {
            format!("Failed to create output directory {}", output_dir.display())
        })?;
        let path = output_dir.join(format!("{stem}.{language}.{extension}"));
        save_subtitle(&path, &subtitle, self.force)?;
        Ok(CliOutput::facet(DownloadReport {
            path: path.display().to_string(),
            release_name,
            language,
            match_score,
        }))
    }
}

fn save_subtitle(path: &Path, subtitle: &[u8], force: bool) -> Result<()> {
    if !force {
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .wrap_err_with(|| format!("Failed to create subtitle file {}", path.display()))?;
        if let Err(error) = output.write_all(subtitle) {
            drop(output);
            let _ = std::fs::remove_file(path);
            return Err(error).wrap_err("Failed to write subtitle file");
        }
        return Ok(());
    }

    let parent = path
        .parent()
        .ok_or_else(|| eyre!("Subtitle output path has no parent directory"))?;
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| eyre!("Subtitle output path has no UTF-8 filename"))?;
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    for attempt in 0..16 {
        let temporary = parent.join(format!(
            ".{filename}.teamy-subs-{}-{timestamp}-{attempt}.tmp",
            std::process::id()
        ));
        let mut output = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
        {
            Ok(output) => output,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error).wrap_err("Failed to stage subtitle file"),
        };
        if let Err(error) = output.write_all(subtitle) {
            drop(output);
            let _ = std::fs::remove_file(&temporary);
            return Err(error).wrap_err("Failed to write subtitle file");
        }
        drop(output);
        let result = std::fs::rename(&temporary, path)
            .wrap_err_with(|| format!("Failed to replace subtitle file {}", path.display()));
        if result.is_err() {
            let _ = std::fs::remove_file(&temporary);
        }
        return result;
    }
    bail!("Failed to find an unused temporary subtitle filename")
}

fn choose_candidate(
    candidates: &[SubtitleCandidate],
    pick: Option<usize>,
) -> Result<&SubtitleCandidate> {
    if let Some(pick) = pick {
        return candidates
            .get(pick.saturating_sub(1))
            .filter(|_| pick > 0)
            .ok_or_else(|| eyre!("--pick must name a candidate shown by subtitle search"));
    }
    let best = candidates
        .iter()
        .max_by(|left, right| {
            left.match_score
                .unwrap_or_default()
                .total_cmp(&right.match_score.unwrap_or_default())
        })
        .ok_or_else(|| eyre!("SubDL found no subtitles"))?;
    let score = best.match_score.unwrap_or_default();
    if score < 0.8 {
        bail!("Best subtitle match scored {score:.2}; run subtitle search and choose with --pick");
    }
    if candidates
        .iter()
        .filter(|candidate| candidate.match_score == best.match_score)
        .count()
        > 1
    {
        bail!(
            "Several subtitles have the same top score; run subtitle search and choose with --pick"
        );
    }
    Ok(best)
}

fn extract_subtitle(archive_bytes: &[u8]) -> Result<(&'static str, Vec<u8>)> {
    let mut archive = ZipArchive::new(Cursor::new(archive_bytes))
        .map_err(|_| eyre!("SubDL download was not a readable ZIP archive"))?;
    let mut best: Option<(usize, &'static str, u8)> = None;
    let mut ambiguous_best = false;
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|_| eyre!("Invalid ZIP entry"))?;
        if entry.is_dir() || entry.size() == 0 || entry.size() > MAX_SUBTITLE_BYTES {
            continue;
        }
        let extension = Path::new(entry.name())
            .extension()
            .and_then(|part| part.to_str())
            .map(str::to_ascii_lowercase);
        let choice = match extension.as_deref() {
            Some("srt") => Some(("srt", 0)),
            Some("ass") => Some(("ass", 1)),
            Some("vtt") => Some(("vtt", 2)),
            _ => None,
        };
        if let Some((extension, priority)) = choice {
            if best.is_none_or(|(_, _, old_priority)| priority < old_priority) {
                best = Some((index, extension, priority));
                ambiguous_best = false;
            } else if best.is_some_and(|(_, _, old_priority)| priority == old_priority) {
                ambiguous_best = true;
            }
        }
    }
    if ambiguous_best {
        bail!("ZIP contains multiple equally preferred subtitle files");
    }
    let (index, extension, _) =
        best.ok_or_else(|| eyre!("ZIP contained no supported subtitle file"))?;
    let mut entry = archive
        .by_index(index)
        .map_err(|_| eyre!("Invalid ZIP entry"))?;
    let mut subtitle = Vec::with_capacity(entry.size() as usize);
    entry
        .read_to_end(&mut subtitle)
        .map_err(|_| eyre!("Failed to extract subtitle file"))?;
    if subtitle.is_empty() || subtitle.len() as u64 > MAX_SUBTITLE_BYTES {
        bail!("Extracted subtitle file has an invalid size");
    }
    Ok((extension, subtitle))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(score: f64) -> SubtitleCandidate {
        SubtitleCandidate {
            name: String::new(),
            release_name: String::new(),
            lang: String::new(),
            language: None,
            match_score: Some(score),
            url: String::new(),
        }
    }

    #[test]
    fn equal_top_scores_require_explicit_pick() {
        let candidates = [candidate(1.0), candidate(1.0)];
        assert!(choose_candidate(&candidates, None).is_err());
        assert!(std::ptr::eq(
            choose_candidate(&candidates, Some(2)).unwrap(),
            &candidates[1]
        ));
    }

    #[test]
    fn low_confidence_requires_explicit_pick() {
        let candidates = [candidate(0.5)];
        assert!(choose_candidate(&candidates, None).is_err());
        assert!(choose_candidate(&candidates, Some(1)).is_ok());
    }

    #[test]
    fn season_pack_with_multiple_subtitle_files_is_ambiguous() {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default();
        writer.start_file("episode-1.srt", options).unwrap();
        writer
            .write_all(b"1\n00:00:01,000 --> 00:00:02,000\nOne\n")
            .unwrap();
        writer.start_file("episode-2.srt", options).unwrap();
        writer
            .write_all(b"1\n00:00:01,000 --> 00:00:02,000\nTwo\n")
            .unwrap();
        let bytes = writer.finish().unwrap().into_inner();
        assert!(extract_subtitle(&bytes).is_err());
    }

    #[test]
    fn force_replaces_existing_subtitle_after_staging() {
        let directory = std::env::temp_dir().join(format!(
            "teamy-subs-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("movie.en.srt");
        std::fs::write(&path, b"old").unwrap();
        assert!(save_subtitle(&path, b"new", false).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"old");
        save_subtitle(&path, b"new", true).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"new");
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }
}
