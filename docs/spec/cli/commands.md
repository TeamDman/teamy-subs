# teamy-subs CLI structure

r[cli.command.download]
The CLI MUST expose a `download` command for fetching subtitle files from remote media sources.

r[cli.command.extract]
The CLI MUST expose an `extract` command for extracting subtitle streams from local media containers.

r[cli.command.convert]
The CLI MUST expose a `convert` command for converting subtitle files between supported formats.

r[cli.global.debug]
The CLI MUST expose a global debug switch that enables verbose diagnostics.

r[cli.global.log-file]
The CLI MUST support writing structured logs to an optional file path.