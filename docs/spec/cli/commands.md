# teamy-subs CLI structure

r[cli.command.download]
The CLI MUST expose a `download` command for fetching subtitle files from remote media sources.

r[cli.command.extract]
The CLI MUST expose an `extract` command for extracting subtitle streams from local media containers.

r[cli.command.convert]
The CLI MUST expose a `convert` command for converting subtitle files between supported formats.

r[cli.command.convert.output.infer-name-from-format]
When the `convert` output argument is a bare known format such as `txt` or `vtt`, the CLI MUST infer the output file name from the input file name and replace only the extension.

r[cli.command.convert.output.explicit-path]
When the `convert` output argument includes an explicit file name such as `episode.txt`, the CLI MUST use that file path as provided.

r[cli.global.debug]
The CLI MUST expose a global debug switch that enables verbose diagnostics.

r[cli.global.log-file]
The CLI MUST support writing structured logs to an optional file path.