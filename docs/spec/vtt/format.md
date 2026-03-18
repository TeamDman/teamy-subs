# WebVTT support

r[vtt.document.header]
The parser MUST recognize the `WEBVTT` file header before processing subtitle content.

r[vtt.document.blocks]
The parser MUST model top-level WebVTT blocks including cues and non-cue blocks such as `NOTE`, `STYLE`, and `REGION`.

r[vtt.cue.identifier]
The parser MUST support cues with optional identifier lines.

r[vtt.cue.timing]
The parser MUST model cue start and end timestamps and preserve cue setting tokens.

r[vtt.payload.fragments]
The parser MUST preserve cue payload text at the fragment level, including timed-text extensions that occur in real-world files.

r[vtt.extensions.partition]
The specification MUST distinguish official WebVTT rules from supported extensions accepted for compatibility.