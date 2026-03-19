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

/// Parse failures for a WebVTT document.
#[derive(Facet, Debug, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum VttParseError {
    MissingHeader,
    InvalidFormat(String),
    InvalidCueTiming(String),
    InvalidCueBlock(String),
    InvalidTimestamp(VttTimestampParseError),
}

impl fmt::Display for VttParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingHeader => write!(f, "missing WEBVTT header"),
            Self::InvalidFormat(message) => write!(f, "invalid format: {message}"),
            Self::InvalidCueTiming(message) => write!(f, "invalid cue timing: {message}"),
            Self::InvalidCueBlock(message) => write!(f, "invalid cue block: {message}"),
            Self::InvalidTimestamp(error) => write!(f, "invalid timestamp: {error}"),
        }
    }
}

impl std::error::Error for VttParseError {}

/// A full WebVTT document shape.
#[derive(Facet, Debug, Clone, PartialEq, Eq, Default)]
pub struct VttDocument {
    pub header: VttHeader,
    pub blocks: Vec<VttBlock>,
}

impl VttDocument {
    /// Parse a WebVTT document from text.
    pub fn parse(content: &str) -> Result<Self, VttParseError> {
        let content = normalize_document_content(content);
        let lines = content.lines().collect::<Vec<_>>();

        let Some(first_line) = lines.first() else {
            return Err(VttParseError::MissingHeader);
        };
        let first_line = strip_bom(first_line).trim();
        if !first_line.starts_with("WEBVTT") {
            return Err(VttParseError::MissingHeader);
        }

        let mut header = VttHeader::default();
        if first_line.len() > 6 {
            let description = first_line[6..].trim();
            if !description.is_empty() {
                header.description = Some(description.to_string());
            }
        }

        let mut index = 1;
        while index < lines.len() {
            let line = lines[index].trim();
            if line.is_empty() {
                index += 1;
                break;
            }

            if !is_header_metadata_line(line) {
                break;
            }

            let Some((key, value)) = line.split_once(':') else {
                return Err(VttParseError::InvalidFormat(format!(
                    "invalid header metadata line: {line}"
                )));
            };
            header.metadata.push(VttMetadataEntry {
                key: key.trim().to_string(),
                value: value.trim().to_string(),
            });
            index += 1;
        }

        let mut blocks = Vec::new();
        while index < lines.len() {
            while index < lines.len() && lines[index].trim().is_empty() {
                index += 1;
            }
            if index >= lines.len() {
                break;
            }

            let (block, next_index) = parse_block(&lines, index)?;
            blocks.push(block);
            index = next_index;
        }

        Ok(Self { header, blocks })
    }

    /// Return the human-readable text projection for this document.
    #[must_use]
    pub fn deduplicated_text(&self) -> String {
        TxtDocument::from_cue_texts(self.blocks.iter().filter_map(|block| match block {
            VttBlock::Cue(cue) => Some(cue.payload_plain_text()),
            _ => None,
        }))
        .to_plain_text()
    }
}

impl FromStr for VttDocument {
    type Err = VttParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl fmt::Display for VttDocument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.header)?;

        if !self.blocks.is_empty() {
            writeln!(f)?;
            writeln!(f)?;
        }

        for (index, block) in self.blocks.iter().enumerate() {
            if index > 0 {
                writeln!(f)?;
                writeln!(f)?;
            }

            write!(f, "{block}")?;
        }

        Ok(())
    }
}

/// Header data for a WebVTT document.
#[derive(Facet, Debug, Clone, PartialEq, Eq, Default)]
pub struct VttHeader {
    pub description: Option<String>,
    pub metadata: Vec<VttMetadataEntry>,
}

impl fmt::Display for VttHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(description) = self.description.as_ref() {
            write!(f, "WEBVTT {description}")?;
        } else {
            write!(f, "WEBVTT")?;
        }

        for entry in &self.metadata {
            writeln!(f)?;
            write!(f, "{}: {}", entry.key, entry.value)?;
        }

        Ok(())
    }
}

/// A metadata key/value pair in the VTT header.
#[derive(Facet, Debug, Clone, PartialEq, Eq)]
pub struct VttMetadataEntry {
    pub key: String,
    pub value: String,
}

impl fmt::Display for VttMetadataEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.key, self.value)
    }
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

impl fmt::Display for VttBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Note(block) => write!(f, "{block}"),
            Self::Style(block) => write!(f, "{block}"),
            Self::Region(block) => write!(f, "{block}"),
            Self::Cue(cue) => write!(f, "{cue}"),
        }
    }
}

/// A NOTE block.
#[derive(Facet, Debug, Clone, PartialEq, Eq)]
pub struct VttNoteBlock {
    pub lines: Vec<String>,
}

impl fmt::Display for VttNoteBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "NOTE")?;
        for line in &self.lines {
            writeln!(f, "{line}")?;
        }
        Ok(())
    }
}

/// A STYLE block.
#[derive(Facet, Debug, Clone, PartialEq, Eq)]
pub struct VttStyleBlock {
    pub css: Vec<String>,
}

impl fmt::Display for VttStyleBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "STYLE")?;
        for line in &self.css {
            writeln!(f, "{line}")?;
        }
        Ok(())
    }
}

/// A REGION block.
#[derive(Facet, Debug, Clone, PartialEq, Eq)]
pub struct VttRegionBlock {
    pub lines: Vec<String>,
}

impl fmt::Display for VttRegionBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "REGION")?;
        for line in &self.lines {
            writeln!(f, "{line}")?;
        }
        Ok(())
    }
}

/// A cue in a WebVTT document.
#[derive(Facet, Debug, Clone, PartialEq, Eq)]
pub struct VttCue {
    pub identifier: Option<String>,
    pub timing: VttCueTiming,
    pub payload: VttCuePayload,
}

impl VttCue {
    /// Return the readable plain-text projection of the cue payload.
    #[must_use]
    pub fn payload_plain_text(&self) -> String {
        self.payload.plain_text()
    }
}

impl fmt::Display for VttCue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(identifier) = self.identifier.as_ref() {
            writeln!(f, "{identifier}")?;
        }

        write!(f, "{}", self.timing)?;
        if !self.payload.lines.is_empty() {
            writeln!(f)?;
            write!(f, "{}", self.payload)?;
        }

        Ok(())
    }
}

/// Timing and settings for a cue.
#[derive(Facet, Debug, Clone, PartialEq, Eq)]
pub struct VttCueTiming {
    pub start: VttTimestamp,
    pub end: VttTimestamp,
    pub settings: Vec<VttCueSetting>,
}

impl fmt::Display for VttCueTiming {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} --> {}", self.start, self.end)?;

        for setting in &self.settings {
            write!(f, " {setting}")?;
        }

        Ok(())
    }
}

/// A single cue setting token.
#[derive(Facet, Debug, Clone, PartialEq, Eq)]
pub struct VttCueSetting {
    pub key: String,
    pub value: String,
}

impl fmt::Display for VttCueSetting {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.value.is_empty() {
            write!(f, "{}", self.key)
        } else {
            write!(f, "{}:{}", self.key, self.value)
        }
    }
}

/// A cue payload broken into lines and fragments.
#[derive(Facet, Debug, Clone, PartialEq, Eq, Default)]
pub struct VttCuePayload {
    pub lines: Vec<VttCuePayloadLine>,
}

impl VttCuePayload {
    /// Convert this payload to plain text.
    #[must_use]
    pub fn plain_text(&self) -> String {
        self.lines
            .iter()
            .map(|line| {
                line.fragments
                    .iter()
                    .map(|fragment| match fragment {
                        VttCueFragment::Text(text)
                        | VttCueFragment::TimestampedText { text, .. }
                        | VttCueFragment::RawTag(text) => text.as_str(),
                    })
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl From<&str> for VttCuePayload {
    fn from(value: &str) -> Self {
        let lines = value
            .lines()
            .map(|line| VttCuePayloadLine {
                fragments: parse_payload_fragments(line),
            })
            .collect();

        Self { lines }
    }
}

impl fmt::Display for VttCuePayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, line) in self.lines.iter().enumerate() {
            if index > 0 {
                writeln!(f)?;
            }
            write!(f, "{line}")?;
        }
        Ok(())
    }
}

/// One rendered line from a cue payload.
#[derive(Facet, Debug, Clone, PartialEq, Eq, Default)]
pub struct VttCuePayloadLine {
    pub fragments: Vec<VttCueFragment>,
}

impl fmt::Display for VttCuePayloadLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for fragment in &self.fragments {
            write!(f, "{fragment}")?;
        }
        Ok(())
    }
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

impl fmt::Display for VttCueFragment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text(text) => write!(f, "{text}"),
            Self::TimestampedText { timestamp, text } => write!(f, "<{timestamp}><c>{text}</c>"),
            Self::RawTag(tag) => write!(f, "{tag}"),
        }
    }
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

impl fmt::Display for TxtLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(timestamp) = self.timestamp {
            write!(f, "[{timestamp}] {}", self.text)
        } else {
            write!(f, "{}", self.text)
        }
    }
}

impl fmt::Display for TxtDocument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, line) in self.lines.iter().enumerate() {
            if index > 0 {
                writeln!(f)?;
            }
            write!(f, "{line}")?;
        }

        Ok(())
    }
}

fn normalize_document_content(content: &str) -> String {
    content.replace("\r\n", "\n").replace('\r', "\n")
}

fn strip_bom(line: &str) -> &str {
    line.strip_prefix('\u{feff}').unwrap_or(line)
}

fn is_header_metadata_line(line: &str) -> bool {
    line.split_once(':').is_some()
}

fn is_block_header_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed == "STYLE" || trimmed == "REGION" || trimmed.starts_with("NOTE")
}

fn is_cue_start_line(lines: &[&str], index: usize) -> bool {
    if index >= lines.len() {
        return false;
    }

    let current = lines[index].trim();
    if current.is_empty() || is_block_header_line(current) {
        return false;
    }

    current.contains("-->") || (index + 1 < lines.len() && lines[index + 1].trim().contains("-->"))
}

fn skip_blank_lines(lines: &[&str], mut index: usize) -> usize {
    while index < lines.len() && lines[index].trim().is_empty() {
        index += 1;
    }
    index
}

fn parse_block(lines: &[&str], index: usize) -> Result<(VttBlock, usize), VttParseError> {
    let line = lines[index].trim_end();
    let trimmed = line.trim();

    if let Some(rest) = trimmed.strip_prefix("NOTE") {
        let mut note_lines = Vec::new();
        if !rest.trim().is_empty() {
            note_lines.push(rest.trim_start().to_string());
        }

        let mut next = index + 1;
        while next < lines.len() && !lines[next].trim().is_empty() {
            note_lines.push(lines[next].to_string());
            next += 1;
        }

        return Ok((VttBlock::Note(VttNoteBlock { lines: note_lines }), next));
    }

    if trimmed == "STYLE" {
        let mut css = Vec::new();
        let mut next = index + 1;
        while next < lines.len() && !lines[next].trim().is_empty() {
            css.push(lines[next].to_string());
            next += 1;
        }
        return Ok((VttBlock::Style(VttStyleBlock { css }), next));
    }

    if trimmed == "REGION" {
        let mut region_lines = Vec::new();
        let mut next = index + 1;
        while next < lines.len() && !lines[next].trim().is_empty() {
            region_lines.push(lines[next].to_string());
            next += 1;
        }
        return Ok((VttBlock::Region(VttRegionBlock { lines: region_lines }), next));
    }

    if trimmed.contains("-->") {
        return parse_cue_block(lines, index, None);
    }

    if index + 1 < lines.len() && lines[index + 1].trim().contains("-->") {
        return parse_cue_block(lines, index, Some(trimmed.to_string()));
    }

    Err(VttParseError::InvalidCueBlock(format!("unrecognized block header: {line}")))
}

fn parse_cue_block(
    lines: &[&str],
    index: usize,
    identifier: Option<String>,
) -> Result<(VttBlock, usize), VttParseError> {
    let mut next = index;
    let (identifier, timing_line) = match identifier {
        Some(identifier) => {
            next += 1;
            (Some(identifier), lines[next].trim())
        }
        None => (None, lines[next].trim()),
    };

    let timing = parse_timing_line(timing_line)?;
    next += 1;

    let mut payload_lines = Vec::new();
    let mut saw_payload_content = false;

    while next < lines.len() {
        if is_cue_start_line(lines, next) || is_block_header_line(lines[next]) {
            break;
        }

        if lines[next].trim().is_empty() {
            let next_non_blank = skip_blank_lines(lines, next);
            if next_non_blank >= lines.len() || is_cue_start_line(lines, next_non_blank) {
                next = next_non_blank;
                break;
            }

            if saw_payload_content {
                payload_lines.push(VttCuePayloadLine::default());
            }
            next = next_non_blank;
            continue;
        }

        payload_lines.push(VttCuePayloadLine {
            fragments: parse_payload_fragments(lines[next]),
        });
        saw_payload_content = true;
        next += 1;
    }

    Ok((
        VttBlock::Cue(VttCue {
            identifier,
            timing,
            payload: VttCuePayload { lines: payload_lines },
        }),
        next,
    ))
}

fn parse_timing_line(line: &str) -> Result<VttCueTiming, VttParseError> {
    let Some((start, rest)) = line.split_once("-->") else {
        return Err(VttParseError::InvalidCueTiming(format!("missing arrow: {line}")));
    };

    let mut tail = rest.split_whitespace();
    let end = tail
        .next()
        .ok_or_else(|| VttParseError::InvalidCueTiming(format!("missing end timestamp: {line}")))?
        .parse::<VttTimestamp>()
        .map_err(VttParseError::InvalidTimestamp)?;

    let start = start
        .trim()
        .parse::<VttTimestamp>()
        .map_err(VttParseError::InvalidTimestamp)?;

    let mut settings = Vec::new();
    for token in tail {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }

        if let Some((key, value)) = token.split_once(':') {
            settings.push(VttCueSetting {
                key: key.trim().to_string(),
                value: value.trim().to_string(),
            });
        } else {
            settings.push(VttCueSetting {
                key: token.to_string(),
                value: String::new(),
            });
        }
    }

    Ok(VttCueTiming { start, end, settings })
}

/// Parse a raw cue payload into semantic fragments.
#[must_use]
pub fn parse_payload_fragments(payload: &str) -> Vec<VttCueFragment> {
    let mut fragments = Vec::new();
    let mut cursor = 0;

    while let Some(start_offset) = payload[cursor..].find('<') {
        let start = cursor + start_offset;
        if start > cursor {
            fragments.push(VttCueFragment::Text(payload[cursor..start].to_string()));
        }

        let Some(end_offset) = payload[start + 1..].find('>') else {
            fragments.push(VttCueFragment::Text(payload[start..].to_string()));
            return fragments;
        };

        let end = start + 1 + end_offset;
        let tag_content = &payload[start + 1..end];
        if let Ok(timestamp) = VttTimestamp::from_str(tag_content) {
            let after_tag = end + 1;
            if payload[after_tag..].starts_with("<c>") {
                let text_start = after_tag + 3;
                if let Some(close_offset) = payload[text_start..].find("</c>") {
                    let text_end = text_start + close_offset;
                    fragments.push(VttCueFragment::TimestampedText {
                        timestamp,
                        text: payload[text_start..text_end].to_string(),
                    });
                    cursor = text_end + 4;
                    continue;
                }
            }

            fragments.push(VttCueFragment::TimestampedText {
                timestamp,
                text: String::new(),
            });
            cursor = after_tag;
            continue;
        }

        fragments.push(VttCueFragment::RawTag(payload[start..=end].to_string()));
        cursor = end + 1;
    }

    if cursor < payload.len() {
        fragments.push(VttCueFragment::Text(payload[cursor..].to_string()));
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
            VttCueFragment::RawTag(_) => String::new(),
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

    #[test]
    fn parses_ytdlp_caption_sample_with_blank_line_between_timing_and_payload() {
        let document = VttDocument::parse(
            "WEBVTT\nKind: captions\nLanguage: en\n\n00:00:07.200 --> 00:00:09.190 align:start position:0%\n\nhello<00:00:07.919><c> everyone</c>\n\n00:00:09.190 --> 00:00:09.200 align:start position:0%\nhello everyone\n",
        )
        .unwrap();

        assert_eq!(document.header.metadata.len(), 2);
        assert_eq!(document.blocks.len(), 2);
        assert_eq!(document.deduplicated_text(), "hello everyone");
    }
}
