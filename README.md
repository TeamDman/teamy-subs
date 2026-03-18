# Teamy-Subs

I'm tired, boss.

Tired of dealing with the same old stuff.

Tired of forgetting which project had which feature.

This project will handle:

- Downloading subtitles for remote media
- Extracting subtitles from container files
- Converting subtitle file formats

## Requirements

- `yt-dlp` on `PATH` for `download`
- `ffmpeg` and `ffprobe` on `PATH` for `extract`

Here's the CLI structure I have in mind:

```
teamy-subs download https://www.youtube.com/watch?v=j2itsZz5PiM # `--output-dir {path}` can be specified; just a yt-dlp wrapper via Command
teamy-subs extract abc.mkv . # just a ffmpeg wrapper via Command
teamy-subs convert ahoy.vtt ahoy.txt # use TeamDman/vtt fork
```

## Implemented commands

```text
teamy-subs download <url> [--output-dir <path>] [--sub-langs <langs>] [--no-auto-subs]
teamy-subs extract <input-file> <output-dir>
teamy-subs convert <input-file> <output-file>
```

Current conversion support:

- `.vtt -> .txt`
- `.vtt -> .vtt` normalization