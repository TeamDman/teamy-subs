# Teamy-Subs

I'm tired, boss.

Tired of dealing with the same old stuff.

Tired of forgetting which project had which feature.

This project will handle:

- Downloading subtitles for remote media
- Extracting subtitles from container files
- Converting subtitle file formats

## Requirements

- `yt-dlp` on `PATH` for `yt-dlp download`
- `ffmpeg` and `ffprobe` on `PATH` for `extract`
- A SubDL API key for `subdl` commands; `op` on `PATH` if using a 1Password reference

Here's the CLI structure I have in mind:

```
teamy-subs yt-dlp download https://www.youtube.com/watch?v=j2itsZz5PiM
teamy-subs extract abc.mkv . # just a ffmpeg wrapper via Command
teamy-subs convert ahoy.vtt ahoy.txt # use TeamDman/vtt fork
teamy-subs convert ahoy.vtt txt      # infers ahoy.txt
```

## Implemented commands

```text
teamy-subs yt-dlp download <url> [--output-dir <path>] [--sub-langs <langs>] [--no-auto-subs]
teamy-subs extract <input-file> <output-dir>
teamy-subs convert <input-file> <output-file>
teamy-subs subdl account show
teamy-subs subdl login [--ttl-minutes 15]
teamy-subs subdl status
teamy-subs subdl logout
teamy-subs subdl title search <query>
teamy-subs subdl subtitle search --file <video-file> [--language en]
teamy-subs subdl subtitle download --file <video-file> [--language en] [--output-dir <dir>] [--pick <number>] [--force]
```

Current conversion support:

- `.vtt -> .txt`
- `.vtt -> .vtt` normalization

For `convert`, the output argument can also be a bare known format such as `txt` or `vtt`.
In that case, the command reuses the input file name and swaps only the extension.

## SubDL authentication and downloads

Run `subdl login` once per short session. Pass `--op-ref` immediately after `subdl`, or set `SUBDL_API_KEY_OP_REF` to a 1Password secret reference such as `op://<vault>/<item>/<field>`. The default lifetime is 15 minutes; `--ttl-minutes` accepts 1 to 60. Search, download, and account commands reuse the lease across processes without reading 1Password again. `subdl status` shows the remaining time, and `subdl logout` deletes the lease early.

The lease is stored outside the repository in a user-private directory, encrypted with user-scoped Windows DPAPI, and scheduled for deletion through Windows Task Scheduler. Expiry is checked whenever a command loads the lease, even if scheduled cleanup has not run. The task receives only a generated lease ID, never the key. `SUBDL_API_KEY` remains available as a process-only alternative; an explicit `--op-ref` on a search or download command makes a one-time 1Password read.

```text
teamy-subs subdl --op-ref "op://<vault>/<item>/<field>" login
teamy-subs subdl status
teamy-subs subdl subtitle search --file movie.mkv --language en
teamy-subs subdl subtitle download --file movie.mkv --language en --output-dir subtitles --pick 1
```

Search shows ranked candidates without their download URLs. SubDL currently puts the API key in returned URLs; the CLI discards the query and sends the key in an `X-API-Key` header instead. Download requires `--pick` when the highest scores tie or the best score is below 0.8. It extracts one `.srt`, `.ass`, or `.vtt` file from the ZIP and names it after the video with the language suffix. It does not overwrite an existing file unless `--force` is set.

Commands return structured output. Use `--output-format text`, `json`, or `csv`; interactive output defaults to text and redirected output defaults to JSON. Ctrl+C requests cancellation. `--stop-after-duration 30s` can also bound a command run.
