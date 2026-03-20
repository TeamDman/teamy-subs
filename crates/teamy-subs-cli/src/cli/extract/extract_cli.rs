use arbitrary::Arbitrary;
use eyre::Context;
use eyre::Result;
use eyre::bail;
use facet::Facet;
use figue::{self as args};
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

/// Extract subtitle streams from a local media container.
// r[impl cli.command.extract]
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct ExtractArgs {
    /// Media file to inspect.
    #[facet(args::positional)]
    pub input_file: String,

    /// Directory where extracted subtitle files should be written.
    #[facet(args::positional)]
    pub output_dir: String,
}

impl ExtractArgs {
    /// # Errors
    ///
    /// This function will return an error if the input cannot be probed, no subtitle
    /// streams are found, or one or more extraction commands fail.
    #[expect(clippy::unused_async)]
    pub async fn invoke(self) -> Result<()> {
        let input_file = PathBuf::from(&self.input_file);
        let output_dir = PathBuf::from(&self.output_dir);

        if !input_file.is_file() {
            bail!("Input media file {} does not exist", input_file.display());
        }

        std::fs::create_dir_all(&output_dir).wrap_err_with(|| {
            format!("Failed to create output directory {}", output_dir.display())
        })?;

        let streams = probe_subtitle_streams(&input_file)?;
        if streams.is_empty() {
            bail!("No subtitle streams were found in {}", input_file.display());
        }

        let base_name = input_file
            .file_stem()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("subtitle");

        let mut failures = Vec::new();
        for (position, stream) in streams.iter().enumerate() {
            let strategy = stream_output_strategy(stream.codec_name.as_deref());
            let output_path = build_output_path(
                &output_dir,
                base_name,
                position + 1,
                stream,
                strategy.extension,
            );

            let mut command = Command::new("ffmpeg");
            command
                .arg("-y")
                .arg("-i")
                .arg(&input_file)
                .arg("-map")
                .arg(format!("0:{}", stream.index));

            match strategy.mode {
                SubtitleExtractionMode::Copy => {
                    command.arg("-c:s").arg("copy");
                }
                SubtitleExtractionMode::EncodeSrt => {
                    command.arg("-c:s").arg("srt");
                }
            }

            let output = command.arg(&output_path).output().wrap_err(
                "Failed to launch ffmpeg. Ensure it is installed and available on PATH.",
            )?;

            if output.status.success() {
                println!("Extracted {}", output_path.display());
            } else {
                failures.push(format!(
                    "stream {}: {}",
                    stream.index,
                    String::from_utf8_lossy(&output.stderr).trim()
                ));
            }
        }

        if failures.is_empty() {
            Ok(())
        } else {
            bail!(
                "Failed to extract one or more subtitle streams:\n{}",
                failures.join("\n")
            );
        }
    }
}

#[derive(Debug, Default)]
struct FfprobeResponse {
    streams: Vec<FfprobeStream>,
}

#[derive(Debug, Default)]
struct FfprobeStream {
    index: usize,
    codec_name: Option<String>,
    tags: Option<FfprobeTags>,
}

#[derive(Debug, Default)]
struct FfprobeTags {
    language: Option<String>,
    title: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SubtitleExtractionMode {
    Copy,
    EncodeSrt,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SubtitleOutputStrategy {
    extension: &'static str,
    mode: SubtitleExtractionMode,
}

fn probe_subtitle_streams(input_file: &Path) -> Result<Vec<FfprobeStream>> {
    let output = Command::new("ffprobe")
        .arg("-v")
        .arg("error")
        .arg("-select_streams")
        .arg("s")
        .arg("-show_entries")
        .arg("stream=index,codec_name:stream_tags=language,title")
        .arg("-of")
        .arg("default=noprint_wrappers=1:nokey=0")
        .arg(input_file)
        .output()
        .wrap_err(
            "Failed to launch ffprobe. Ensure ffmpeg/ffprobe are installed and available on PATH.",
        )?;

    if !output.status.success() {
        bail!(
            "ffprobe exited unsuccessfully: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    parse_ffprobe_streams(&String::from_utf8_lossy(&output.stdout))
}

fn parse_ffprobe_streams(output: &str) -> Result<Vec<FfprobeStream>> {
    let mut response = FfprobeResponse::default();
    let mut current = FfprobeStream::default();
    let mut saw_field = false;

    for raw_line in output.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            if saw_field {
                if current.index == 0 {
                    bail!("ffprobe output was missing a stream index");
                }
                response.streams.push(std::mem::take(&mut current));
                saw_field = false;
            }

            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        saw_field = true;
        match key {
            "index" => {
                current.index = value
                    .parse::<usize>()
                    .wrap_err_with(|| format!("Invalid ffprobe stream index: {value}"))?;
            }
            "codec_name" => {
                current.codec_name = if value.is_empty() {
                    None
                } else {
                    Some(value.to_string())
                };
            }
            "TAG:language" => {
                current.tags.get_or_insert_with(Default::default).language = if value.is_empty() {
                    None
                } else {
                    Some(value.to_string())
                };
            }
            "TAG:title" => {
                current.tags.get_or_insert_with(Default::default).title = if value.is_empty() {
                    None
                } else {
                    Some(value.to_string())
                };
            }
            _ => {}
        }
    }

    if saw_field {
        if current.index == 0 {
            bail!("ffprobe output was missing a stream index");
        }
        response.streams.push(current);
    }

    Ok(response.streams)
}

fn stream_output_strategy(codec_name: Option<&str>) -> SubtitleOutputStrategy {
    match codec_name {
        Some("webvtt") => SubtitleOutputStrategy {
            extension: "vtt",
            mode: SubtitleExtractionMode::Copy,
        },
        Some("subrip") => SubtitleOutputStrategy {
            extension: "srt",
            mode: SubtitleExtractionMode::Copy,
        },
        Some("ass") => SubtitleOutputStrategy {
            extension: "ass",
            mode: SubtitleExtractionMode::Copy,
        },
        Some("ssa") => SubtitleOutputStrategy {
            extension: "ssa",
            mode: SubtitleExtractionMode::Copy,
        },
        Some("hdmv_pgs_subtitle") => SubtitleOutputStrategy {
            extension: "sup",
            mode: SubtitleExtractionMode::Copy,
        },
        Some("dvd_subtitle") | Some("xsub") => SubtitleOutputStrategy {
            extension: "sub",
            mode: SubtitleExtractionMode::Copy,
        },
        _ => SubtitleOutputStrategy {
            extension: "srt",
            mode: SubtitleExtractionMode::EncodeSrt,
        },
    }
}

fn build_output_path(
    output_dir: &Path,
    base_name: &str,
    position: usize,
    stream: &FfprobeStream,
    extension: &str,
) -> PathBuf {
    let mut name = format!(
        "{}.subtitle-{:02}",
        sanitize_filename_component(base_name),
        position
    );

    if let Some(language) = stream
        .tags
        .as_ref()
        .and_then(|tags| tags.language.as_deref())
        .and_then(sanitize_optional_component)
    {
        name.push('.');
        name.push_str(&language);
    }

    if let Some(title) = stream
        .tags
        .as_ref()
        .and_then(|tags| tags.title.as_deref())
        .and_then(sanitize_optional_component)
    {
        name.push('.');
        name.push_str(&title);
    }

    output_dir.join(format!("{name}.{extension}"))
}

fn sanitize_optional_component(value: &str) -> Option<String> {
    let sanitized = sanitize_filename_component(value);
    if sanitized.is_empty() {
        None
    } else {
        Some(sanitized)
    }
}

fn sanitize_filename_component(value: &str) -> String {
    let mut sanitized = String::new();
    let mut last_was_separator = false;

    for character in value.trim().chars() {
        let normalized = if character.is_ascii_alphanumeric() {
            Some(character.to_ascii_lowercase())
        } else if matches!(character, '-' | '_' | '.') {
            Some(character)
        } else {
            None
        };

        match normalized {
            Some(character) => {
                sanitized.push(character);
                last_was_separator = false;
            }
            None if !last_was_separator && !sanitized.is_empty() => {
                sanitized.push('-');
                last_was_separator = true;
            }
            None => {}
        }
    }

    sanitized.trim_matches(['-', '.', '_']).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_copy_for_common_text_and_bitmap_codecs() {
        assert_eq!(
            stream_output_strategy(Some("webvtt")),
            SubtitleOutputStrategy {
                extension: "vtt",
                mode: SubtitleExtractionMode::Copy,
            }
        );
        assert_eq!(
            stream_output_strategy(Some("hdmv_pgs_subtitle")),
            SubtitleOutputStrategy {
                extension: "sup",
                mode: SubtitleExtractionMode::Copy,
            }
        );
    }

    #[test]
    fn sanitizes_filename_components() {
        assert_eq!(
            sanitize_filename_component("English (SDH) / Main"),
            "english-sdh-main"
        );
    }

    #[test]
    fn parses_default_ffprobe_stream_listing() {
        let streams = parse_ffprobe_streams(
            "index=2\ncodec_name=webvtt\nTAG:language=eng\nTAG:title=English SDH\n\nindex=4\ncodec_name=subrip\nTAG:language=jpn\n",
        )
        .unwrap();

        assert_eq!(streams.len(), 2);
        assert_eq!(streams[0].index, 2);
        assert_eq!(streams[0].codec_name.as_deref(), Some("webvtt"));
        assert_eq!(streams[0].tags.as_ref().and_then(|tags| tags.language.as_deref()), Some("eng"));
        assert_eq!(streams[0].tags.as_ref().and_then(|tags| tags.title.as_deref()), Some("English SDH"));
        assert_eq!(streams[1].index, 4);
        assert_eq!(streams[1].codec_name.as_deref(), Some("subrip"));
        assert_eq!(streams[1].tags.as_ref().and_then(|tags| tags.language.as_deref()), Some("jpn"));
    }
}
