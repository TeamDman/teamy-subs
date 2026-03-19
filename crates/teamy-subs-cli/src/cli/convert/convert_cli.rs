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
use teamy_facet_vtt::VttDocument;

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

fn read_vtt(path: &Path) -> Result<VttDocument> {
    let content =
        fs::read_to_string(path).wrap_err_with(|| format!("Failed to read {}", path.display()))?;
    VttDocument::parse(&content)
        .wrap_err_with(|| format!("Failed to parse VTT file {}", path.display()))
}

fn convert_vtt_to_text(path: &Path) -> Result<String> {
    let vtt = read_vtt(path)?;
    Ok(vtt.deduplicated_text())
}

fn normalize_vtt(path: &Path) -> Result<String> {
    Ok(read_vtt(path)?.to_string())
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
        let vtt = VttDocument::parse(
            "WEBVTT\n\n00:00:00.000 --> 00:00:01.000\nhello\nworld\n\n00:00:01.000 --> 00:00:02.000\nworld\nagain\n",
        )
        .unwrap();

        assert_eq!(vtt.deduplicated_text(), "hello\nworld\nagain");
    }

    #[test]
    fn removes_character_level_overlap_between_cues() {
        let vtt = VttDocument::parse(
            "WEBVTT\n\n00:00:00.000 --> 00:00:01.000\nhel\n\n00:00:01.000 --> 00:00:02.000\nhello\n",
        )
        .unwrap();

        assert_eq!(vtt.deduplicated_text(), "hello");
    }

    #[test]
    fn strips_timed_cue_payload_markup_like_the_fork() {
        let vtt = VttDocument::parse(
            "WEBVTT\n\n00:00:00.000 --> 00:00:02.000\nwhen<00:00:00.199><c> I</c><00:00:00.280><c> started</c>\n",
        )
        .unwrap();

        assert_eq!(vtt.deduplicated_text(), "when I started");
    }

    #[test]
    fn does_not_insert_separator_when_cues_already_have_spacing() {
        let vtt = VttDocument::parse(
            "WEBVTT\n\n00:00:00.000 --> 00:00:01.000\nHello \n\n00:00:01.000 --> 00:00:02.000\nworld\n",
        )
        .unwrap();

        assert_eq!(vtt.deduplicated_text(), "Hello world");
    }


    #[test]
    fn converts_real_ytdlp_sample_with_blank_lines_between_timing_and_payload() {
        let temp_dir = std::env::temp_dir();
        let input_path = temp_dir.join("teamy-subs-ytdlp-sample.vtt");
        fs::write(
            &input_path,
            "WEBVTT\nKind: captions\nLanguage: en\n\n00:00:07.200 --> 00:00:09.190 align:start position:0%\n\nhello<00:00:07.919><c> everyone</c>\n\n00:00:09.190 --> 00:00:09.200 align:start position:0%\nhello everyone\n",
        )
        .unwrap();

        let output = convert_vtt_to_text(&input_path).unwrap();
        assert_eq!(output, "hello everyone");

        let _ = fs::remove_file(input_path);
    }
}
