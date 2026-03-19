use facet::Facet;
use std::fmt;
use std::str::FromStr;

/// Represents a timestamp in WebVTT timestamp syntax.
#[derive(Facet, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VttTimestamp {
    total_millis: u64,
}

impl VttTimestamp {
    /// Create a timestamp from total milliseconds.
    #[must_use]
    pub const fn from_total_millis(total_millis: u64) -> Self {
        Self { total_millis }
    }

    /// Return the timestamp as total milliseconds.
    #[must_use]
    pub const fn total_millis(self) -> u64 {
        self.total_millis
    }
}

impl fmt::Display for VttTimestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hours = self.total_millis / 3_600_000;
        let minutes = (self.total_millis % 3_600_000) / 60_000;
        let seconds = (self.total_millis % 60_000) / 1_000;
        let millis = self.total_millis % 1_000;

        write!(
            f,
            "{:02}:{:02}:{:02}.{:03}",
            hours, minutes, seconds, millis
        )
    }
}

impl FromStr for VttTimestamp {
    type Err = VttTimestampParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let mut parts = value.split(':');
        let first = parts.next().ok_or(VttTimestampParseError::InvalidFormat)?;
        let second = parts.next().ok_or(VttTimestampParseError::InvalidFormat)?;
        let third = parts.next();

        let total_millis = match third {
            Some(third_part) => {
                let hours = first
                    .parse::<u64>()
                    .map_err(|_| VttTimestampParseError::InvalidHours)?;
                let minutes = second
                    .parse::<u64>()
                    .map_err(|_| VttTimestampParseError::InvalidMinutes)?;
                let (seconds, millis) = parse_seconds_ms(third_part)?;
                hours * 3_600_000 + minutes * 60_000 + seconds * 1_000 + millis
            }
            None => {
                let minutes = first
                    .parse::<u64>()
                    .map_err(|_| VttTimestampParseError::InvalidMinutes)?;
                let (seconds, millis) = parse_seconds_ms(second)?;
                minutes * 60_000 + seconds * 1_000 + millis
            }
        };

        Ok(Self::from_total_millis(total_millis))
    }
}

/// Parse failures for a WebVTT timestamp.
#[derive(Facet, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum VttTimestampParseError {
    InvalidFormat,
    InvalidHours,
    InvalidMinutes,
    InvalidSeconds,
    InvalidMilliseconds,
}

impl fmt::Display for VttTimestampParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFormat => write!(f, "invalid timestamp format"),
            Self::InvalidHours => write!(f, "invalid hours"),
            Self::InvalidMinutes => write!(f, "invalid minutes"),
            Self::InvalidSeconds => write!(f, "invalid seconds"),
            Self::InvalidMilliseconds => write!(f, "invalid milliseconds"),
        }
    }
}

fn parse_seconds_ms(seconds_str: &str) -> Result<(u64, u64), VttTimestampParseError> {
    if let Some(dot_index) = seconds_str.find('.') {
        let seconds = seconds_str[..dot_index]
            .parse::<u64>()
            .map_err(|_| VttTimestampParseError::InvalidSeconds)?;
        let millis_str = &seconds_str[dot_index + 1..];
        let millis_str = match millis_str.len() {
            0 => return Err(VttTimestampParseError::InvalidMilliseconds),
            1 => format!("{millis_str}00"),
            2 => format!("{millis_str}0"),
            _ => millis_str[..3].to_string(),
        };
        let millis = millis_str
            .parse::<u64>()
            .map_err(|_| VttTimestampParseError::InvalidMilliseconds)?;
        Ok((seconds, millis))
    } else {
        let seconds = seconds_str
            .parse::<u64>()
            .map_err(|_| VttTimestampParseError::InvalidSeconds)?;
        Ok((seconds, 0))
    }
}

/// A full WebVTT document shape.
#[derive(Facet, Debug, Clone, PartialEq, Eq, Default)]
pub struct VttDocument {
    pub header: VttHeader,
    pub blocks: Vec<VttBlock>,
}

/// Header data for a WebVTT document.
#[derive(Facet, Debug, Clone, PartialEq, Eq, Default)]
pub struct VttHeader {
    pub description: Option<String>,
    pub metadata: Vec<VttMetadataEntry>,
}

/// A metadata key/value pair in the VTT header.
#[derive(Facet, Debug, Clone, PartialEq, Eq)]
pub struct VttMetadataEntry {
    pub key: String,
    pub value: String,
}

/// A top-level block in a WebVTT file.
#[derive(Facet, Debug, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum VttBlock {
    Note(VttNoteBlock),
    Style(VttStyleBlock),
    Region(VttRegionBlock),
    Cue(VttCue),
}

/// A NOTE block.
#[derive(Facet, Debug, Clone, PartialEq, Eq)]
pub struct VttNoteBlock {
    pub lines: Vec<String>,
}

/// A STYLE block.
#[derive(Facet, Debug, Clone, PartialEq, Eq)]
pub struct VttStyleBlock {
    pub css: Vec<String>,
}

/// A REGION block.
#[derive(Facet, Debug, Clone, PartialEq, Eq)]
pub struct VttRegionBlock {
    pub lines: Vec<String>,
}

/// A cue in a WebVTT document.
#[derive(Facet, Debug, Clone, PartialEq, Eq)]
pub struct VttCue {
    pub identifier: Option<String>,
    pub timing: VttCueTiming,
    pub payload: VttCuePayload,
}

/// Timing and settings for a cue.
#[derive(Facet, Debug, Clone, PartialEq, Eq)]
pub struct VttCueTiming {
    pub start: VttTimestamp,
    pub end: VttTimestamp,
    pub settings: Vec<VttCueSetting>,
}

/// A single cue setting token.
#[derive(Facet, Debug, Clone, PartialEq, Eq)]
pub struct VttCueSetting {
    pub key: String,
    pub value: String,
}

/// A cue payload broken into lines and fragments.
#[derive(Facet, Debug, Clone, PartialEq, Eq, Default)]
pub struct VttCuePayload {
    pub lines: Vec<VttCuePayloadLine>,
}

/// One rendered line from a cue payload.
#[derive(Facet, Debug, Clone, PartialEq, Eq, Default)]
pub struct VttCuePayloadLine {
    pub fragments: Vec<VttCueFragment>,
}

/// A semantic cue payload fragment.
#[derive(Facet, Debug, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum VttCueFragment {
    Text(String),
    TimestampedText {
        timestamp: VttTimestamp,
        text: String,
    },
    RawTag(String),
}

/// A readable TXT document shape.
#[derive(Facet, Debug, Clone, PartialEq, Eq, Default)]
pub struct TxtDocument {
    pub lines: Vec<TxtLine>,
}

impl TxtDocument {
    /// Build a readable TXT document from cue text chunks, applying overlap removal.
    #[must_use]
    pub fn from_cue_texts<I, S>(cue_texts: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut result = String::new();

        for cue_text in cue_texts {
            let cue_text = cue_text.as_ref();
            if cue_text.is_empty() {
                continue;
            }

            let overlap = longest_char_boundary_overlap(&result, cue_text);
            if overlap == 0 && should_insert_separator(&result, cue_text) {
                result.push('\n');
            }
            result.push_str(&cue_text[overlap..]);
        }

        Self {
            lines: result
                .lines()
                .map(|line| TxtLine {
                    timestamp: None,
                    text: line.to_string(),
                })
                .collect(),
        }
    }

    /// Render this document as newline-delimited readable text.
    #[must_use]
    pub fn to_plain_text(&self) -> String {
        self.lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// A single TXT line with optional coarse timestamp metadata.
#[derive(Facet, Debug, Clone, PartialEq, Eq)]
pub struct TxtLine {
    pub timestamp: Option<VttTimestamp>,
    pub text: String,
}

/// Parse a raw cue payload into semantic fragments.
#[must_use]
pub fn parse_payload_fragments(payload: &str) -> Vec<VttCueFragment> {
    let mut fragments = Vec::new();
    let mut rest = payload;

    while let Some(start_index) = rest.find('<') {
        if start_index > 0 {
            let literal = &rest[..start_index];
            if !literal.trim().is_empty() {
                fragments.push(VttCueFragment::Text(literal.to_string()));
            }
        }

        rest = &rest[start_index..];

        if let Some(end_index) = rest.find('>') {
            let tag_content = &rest[1..end_index];
            if let Ok(timestamp) = VttTimestamp::from_str(tag_content) {
                rest = &rest[end_index + 1..];
                if rest.starts_with("<c>") {
                    if let Some(close_index) = rest.find("</c>") {
                        let text = &rest[3..close_index];
                        fragments.push(VttCueFragment::TimestampedText {
                            timestamp,
                            text: text.to_string(),
                        });
                        rest = &rest[close_index + 4..];
                    } else {
                        fragments.push(VttCueFragment::Text(rest.to_string()));
                        break;
                    }
                } else {
                    fragments.push(VttCueFragment::TimestampedText {
                        timestamp,
                        text: String::new(),
                    });
                }
            } else {
                fragments.push(VttCueFragment::Text("<".to_string()));
                rest = &rest[1..];
            }
        } else {
            fragments.push(VttCueFragment::Text(rest.to_string()));
            break;
        }
    }

    if !rest.trim().is_empty() {
        fragments.push(VttCueFragment::Text(rest.to_string()));
    }

    fragments
}

/// Convert a raw cue payload into readable plain text.
#[must_use]
pub fn cue_payload_plain_text(payload: &str) -> String {
    parse_payload_fragments(payload)
        .into_iter()
        .map(|fragment| match fragment {
            VttCueFragment::Text(text) | VttCueFragment::TimestampedText { text, .. } => text,
            VttCueFragment::RawTag(tag) => tag,
        })
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_webvtt_timestamp() {
        let timestamp = VttTimestamp::from_str("01:23:45.678").unwrap();
        assert_eq!(timestamp.total_millis(), 5_025_678);
        assert_eq!(timestamp.to_string(), "01:23:45.678");
    }

    #[test]
    fn parses_short_webvtt_timestamp() {
        let timestamp = VttTimestamp::from_str("23:45.67").unwrap();
        assert_eq!(timestamp.total_millis(), 1_425_670);
        assert_eq!(timestamp.to_string(), "00:23:45.670");
    }

    #[test]
    fn shapes_can_model_vtt_and_txt_documents() {
        let document = VttDocument {
            header: VttHeader {
                description: Some("Sample".to_string()),
                metadata: vec![VttMetadataEntry {
                    key: "Language".to_string(),
                    value: "en".to_string(),
                }],
            },
            blocks: vec![VttBlock::Cue(VttCue {
                identifier: Some("cue-1".to_string()),
                timing: VttCueTiming {
                    start: VttTimestamp::from_total_millis(0),
                    end: VttTimestamp::from_total_millis(2_000),
                    settings: vec![VttCueSetting {
                        key: "align".to_string(),
                        value: "middle".to_string(),
                    }],
                },
                payload: VttCuePayload {
                    lines: vec![VttCuePayloadLine {
                        fragments: vec![VttCueFragment::Text("Hello".to_string())],
                    }],
                },
            })],
        };

        let txt = TxtDocument {
            lines: vec![TxtLine {
                timestamp: Some(VttTimestamp::from_total_millis(0)),
                text: "Hello".to_string(),
            }],
        };

        assert_eq!(document.blocks.len(), 1);
        assert_eq!(txt.lines.len(), 1);
    }

    #[test]
    fn payload_plain_text_strips_timed_markup() {
        assert_eq!(
            cue_payload_plain_text("when<00:00:00.199><c> I</c><00:00:00.280><c> started</c>"),
            "when I started"
        );
    }

    #[test]
    fn txt_document_from_cues_deduplicates_overlap() {
        let txt = TxtDocument::from_cue_texts(["hello", "hello world", "world again"]);
        assert_eq!(txt.to_plain_text(), "hello world again");
    }
}
