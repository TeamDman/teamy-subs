# TXT subtitle projection

r[txt.document.lines]
TXT output MUST be modeled as ordered lines rather than as an opaque blob of text.

r[txt.line.timestamp-prefix]
TXT lines SHOULD support an optional coarse timestamp prefix so users can locate the corresponding point in the media.

r[txt.import.graceful-untimed]
TXT input without timing information MUST still be handled gracefully by the conversion pipeline.

r[txt.readability.first]
TXT output MUST prioritize human readability over lossless preservation of subtitle markup.