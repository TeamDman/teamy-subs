use arbitrary::Arbitrary;
use eyre::Context;
use eyre::Result;
use eyre::bail;
use eyre::eyre;
use facet::Facet;
use figue::{self as args};
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;
use teamy_facet_vtt::VttTimestamp;
use vtt::prelude::WebVtt;

/// Convert subtitle files into other formats.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct ConvertArgs {
    /// Input subtitle file.
    #[facet(args::positional)]
    pub input_file: String,

    /// Output file path.
    #[facet(args::positional)]
    pub output_file: String,
}

impl ConvertArgs {
    /// # Errors
    ///
    /// This function will return an error if the input cannot be read, the subtitle
    /// file cannot be parsed, or the target format is unsupported.
    #[expect(clippy::unused_async)]
    pub async fn invoke(self) -> Result<()> {
        let input_file = PathBuf::from(&self.input_file);
        let output_file = PathBuf::from(&self.output_file);
        let input_extension = lowercase_extension(&input_file)?;
        let output_extension = lowercase_extension(&output_file)?;

        let converted = match (input_extension.as_str(), output_extension.as_str()) {
            ("vtt", "txt") => convert_vtt_to_text(&input_file)?,
            ("vtt", "vtt") => normalize_vtt(&input_file)?,
            _ => bail!(
                "Unsupported conversion from .{input_extension} to .{output_extension}. Supported conversions: .vtt -> .txt, .vtt -> .vtt"
            ),
        };

        if let Some(parent) = output_file.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent).wrap_err_with(|| {
                format!("Failed to create output directory {}", parent.display())
            })?;
        }

        fs::write(&output_file, converted)
            .wrap_err_with(|| format!("Failed to write output file {}", output_file.display()))?;
        println!("Wrote {}", output_file.display());
        Ok(())
    }
}

fn lowercase_extension(path: &Path) -> Result<String> {
    let extension = path
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| eyre!("{} has no file extension", path.display()))?;
    Ok(extension)
}

fn read_vtt(path: &Path) -> Result<WebVtt> {
    let content =
        fs::read_to_string(path).wrap_err_with(|| format!("Failed to read {}", path.display()))?;
    WebVtt::from_str(&content)
        .wrap_err_with(|| format!("Failed to parse VTT file {}", path.display()))
}

fn convert_vtt_to_text(path: &Path) -> Result<String> {
    let content =
        fs::read_to_string(path).wrap_err_with(|| format!("Failed to read {}", path.display()))?;

    match WebVtt::from_str(&content) {
        Ok(vtt) => Ok(deduplicated_text(&vtt)),
        Err(_) => convert_vtt_content_to_text_fallback(&content),
    }
}

fn normalize_vtt(path: &Path) -> Result<String> {
    Ok(read_vtt(path)?.to_string())
}

fn convert_vtt_content_to_text_fallback(content: &str) -> Result<String> {
    let normalized = normalize_vtt_content(content);
    let lines = normalized.lines().collect::<Vec<_>>();

    let Some(first_line) = lines.first() else {
        bail!("Invalid format");
    };
    if !strip_bom(first_line).trim_start().starts_with("WEBVTT") {
        bail!("Invalid format");
    }

    let mut cue_texts = Vec::new();
    let mut index = 1;

    while index < lines.len() {
        if lines[index].trim().is_empty() {
            index += 1;
            continue;
        }

        if is_vtt_block_header(lines[index]) {
            index = skip_block(&lines, index + 1);
            continue;
        }

        if is_timing_line(lines[index]) {
            index += 1;
            let (payload, next_index) = collect_cue_payload(&lines, index);
            if !payload.is_empty() {
                cue_texts.push(cue_payload_plain_text(&payload));
            }
            index = next_index;
            continue;
        }

        if index + 1 < lines.len() && is_timing_line(lines[index + 1]) {
            index += 2;
            let (payload, next_index) = collect_cue_payload(&lines, index);
            if !payload.is_empty() {
                cue_texts.push(cue_payload_plain_text(&payload));
            }
            index = next_index;
            continue;
        }

        index += 1;
    }

    Ok(deduplicated_cue_texts(cue_texts))
}

fn normalize_vtt_content(content: &str) -> String {
    content.replace("\r\n", "\n").replace('\r', "\n")
}

fn strip_bom(line: &str) -> &str {
    line.strip_prefix('\u{feff}').unwrap_or(line)
}

fn is_timing_line(line: &str) -> bool {
    line.contains("-->")
}

fn is_vtt_block_header(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed == "STYLE" || trimmed == "REGION" || trimmed.starts_with("NOTE")
}

fn skip_block(lines: &[&str], mut index: usize) -> usize {
    while index < lines.len() && !lines[index].trim().is_empty() {
        index += 1;
    }
    index
}

fn collect_cue_payload(lines: &[&str], mut index: usize) -> (String, usize) {
    let mut payload_lines = Vec::new();

    while index < lines.len() {
        let current = lines[index];
        let trimmed = current.trim();

        if is_vtt_block_header(current) || is_timing_line(current) {
            break;
        }

        if trimmed.is_empty() {
            let next_index = skip_blank_lines(lines, index);
            if next_index >= lines.len()
                || is_vtt_block_header(lines[next_index])
                || is_timing_line(lines[next_index])
                || (next_index + 1 < lines.len() && is_timing_line(lines[next_index + 1]))
            {
                index = next_index;
                break;
            }

            payload_lines.push(String::new());
            index += 1;
            continue;
        }

        payload_lines.push(current.trim_end().to_string());
        index += 1;
    }

    (payload_lines.join("\n"), index)
}

fn skip_blank_lines(lines: &[&str], mut index: usize) -> usize {
    while index < lines.len() && lines[index].trim().is_empty() {
        index += 1;
    }
    index
}

fn deduplicated_cue_texts(cue_texts: Vec<String>) -> String {
    let mut result = String::new();

    for cue_text in cue_texts {
        if cue_text.is_empty() {
            continue;
        }

        let overlap = longest_char_boundary_overlap(&result, &cue_text);
        if overlap == 0 && should_insert_separator(&result, &cue_text) {
            result.push('\n');
        }
        result.push_str(&cue_text[overlap..]);
    }

    result
}

fn deduplicated_text(vtt: &WebVtt) -> String {
    let mut result = String::new();

    for cue in &vtt.cues {
        let cue_text = cue_payload_plain_text(&cue.payload);
        if cue_text.is_empty() {
            continue;
        }

        let overlap = longest_char_boundary_overlap(&result, &cue_text);
        if overlap == 0 && should_insert_separator(&result, &cue_text) {
            result.push('\n');
        }
        result.push_str(&cue_text[overlap..]);
    }

    result
}

fn cue_payload_plain_text(payload: &str) -> String {
    parse_cue_payload(payload)
        .into_iter()
        .map(|fragment| match fragment {
            CueTextFragment::Text(text) | CueTextFragment::TimedText { text, .. } => text,
        })
        .collect()
}

fn parse_cue_payload(payload: &str) -> Vec<CueTextFragment> {
    let mut fragments = Vec::new();
    let mut rest = payload;

    while let Some(start_idx) = rest.find('<') {
        if start_idx > 0 {
            let literal = &rest[..start_idx];
            if !literal.trim().is_empty() {
                fragments.push(CueTextFragment::Text(literal.to_string()));
            }
        }

        rest = &rest[start_idx..];

        if let Some(end_idx) = rest.find('>') {
            let tag_content = &rest[1..end_idx];
            if let Ok(timestamp) = VttTimestamp::from_str(tag_content) {
                rest = &rest[end_idx + 1..];
                if rest.starts_with("<c>") {
                    if let Some(close_idx) = rest.find("</c>") {
                        let text = &rest[3..close_idx];
                        fragments.push(CueTextFragment::TimedText {
                            timestamp,
                            text: text.to_string(),
                        });
                        rest = &rest[close_idx + 4..];
                    } else {
                        fragments.push(CueTextFragment::Text(rest.to_string()));
                        break;
                    }
                } else {
                    fragments.push(CueTextFragment::TimedText {
                        timestamp,
                        text: String::new(),
                    });
                }
            } else {
                fragments.push(CueTextFragment::Text("<".to_string()));
                rest = &rest[1..];
            }
        } else {
            fragments.push(CueTextFragment::Text(rest.to_string()));
            break;
        }
    }

    if !rest.trim().is_empty() {
        fragments.push(CueTextFragment::Text(rest.to_string()));
    }

    fragments
}

fn longest_char_boundary_overlap(existing: &str, incoming: &str) -> usize {
    let mut valid_indices = incoming
        .char_indices()
        .map(|(index, _)| index)
        .collect::<Vec<usize>>();
    valid_indices.push(incoming.len());

    for &index in valid_indices.iter().rev() {
        if existing.ends_with(&incoming[..index]) {
            return index;
        }
    }

    0
}

fn should_insert_separator(existing: &str, incoming: &str) -> bool {
    let Some(existing_last) = existing.chars().last() else {
        return false;
    };
    let Some(incoming_first) = incoming.chars().next() else {
        return false;
    };

    !existing_last.is_whitespace() && !incoming_first.is_whitespace()
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CueTextFragment {
    Text(String),
    TimedText {
        timestamp: VttTimestamp,
        text: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowercases_extensions() {
        let extension = lowercase_extension(Path::new("movie.VTT")).unwrap();
        assert_eq!(extension, "vtt");
    }

    #[test]
    fn converts_basic_vtt_to_text() {
        let temp_dir = std::env::temp_dir();
        let input_path = temp_dir.join("teamy-subs-convert-test.vtt");
        fs::write(
            &input_path,
            "WEBVTT\n\n00:00:00.000 --> 00:00:01.000\nHello\n\n00:00:01.000 --> 00:00:02.000\nworld\n",
        )
        .unwrap();

        let output = convert_vtt_to_text(&input_path).unwrap();
        assert_eq!(output, "Hello\nworld");

        let _ = fs::remove_file(input_path);
    }

    #[test]
    fn removes_overlapping_lines_between_cues() {
        let vtt = WebVtt::from_str(
            "WEBVTT\n\n00:00:00.000 --> 00:00:01.000\nhello\nworld\n\n00:00:01.000 --> 00:00:02.000\nworld\nagain\n",
        )
        .unwrap();

        assert_eq!(deduplicated_text(&vtt), "hello\nworld\nagain");
    }

    #[test]
    fn removes_character_level_overlap_between_cues() {
        let vtt = WebVtt::from_str(
            "WEBVTT\n\n00:00:00.000 --> 00:00:01.000\nhel\n\n00:00:01.000 --> 00:00:02.000\nhello\n",
        )
        .unwrap();

        assert_eq!(deduplicated_text(&vtt), "hello");
    }

    #[test]
    fn strips_timed_cue_payload_markup_like_the_fork() {
        let vtt = WebVtt::from_str(
            "WEBVTT\n\n00:00:00.000 --> 00:00:02.000\nwhen<00:00:00.199><c> I</c><00:00:00.280><c> started</c>\n",
        )
        .unwrap();

        assert_eq!(deduplicated_text(&vtt), "when I started");
    }

    #[test]
    fn does_not_insert_separator_when_cues_already_have_spacing() {
        let vtt = WebVtt::from_str(
            "WEBVTT\n\n00:00:00.000 --> 00:00:01.000\nHello \n\n00:00:01.000 --> 00:00:02.000\nworld\n",
        )
        .unwrap();

        assert_eq!(deduplicated_text(&vtt), "Hello world");
    }

    #[test]
    fn fallback_handles_note_blocks() {
        let text = convert_vtt_content_to_text_fallback(
            "WEBVTT\n\nNOTE language: en\nGenerated by something\n\n00:00:00.000 --> 00:00:01.000\nHello\n\n00:00:01.000 --> 00:00:02.000\nworld\n",
        )
        .unwrap();

        assert_eq!(text, "Hello\nworld");
    }

    #[test]
    fn fallback_handles_identifier_cues() {
        let text = convert_vtt_content_to_text_fallback(
            "WEBVTT\n\nabc123\n00:00:00.000 --> 00:00:01.000\nHello\n",
        )
        .unwrap();

        assert_eq!(text, "Hello");
    }
}
