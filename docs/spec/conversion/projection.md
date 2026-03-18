# Subtitle conversion semantics

r[conversion.vtt-to-txt.readable-output]
Converting VTT to TXT MUST produce human-readable deduplicated text.

r[conversion.vtt-to-txt.deduplication]
Readable text projection MUST avoid repeating overlapping subtitle content between adjacent cues.

r[conversion.vtt-to-txt.timestamps]
Readable text projection MUST have a defined policy for line-level timestamp prefixes.

r[conversion.vtt-to-vtt.normalization]
Converting VTT to VTT MUST have a defined normalization policy.

r[conversion.sample-corpus]
The supported behavior MUST be validated against a repository fixture corpus drawn from formal examples and real tool outputs.