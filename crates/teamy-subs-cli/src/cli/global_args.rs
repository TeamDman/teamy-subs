//! Global arguments that apply to all commands.

use crate::cli::output::OutputFormat;
use arbitrary::Arbitrary;
use facet::Facet;
use figue::{self as args};
use teamy_cancellation::StopAfterArgs;

/// Global arguments that apply to all commands.
#[derive(Facet, Arbitrary, Debug, Default, PartialEq)]
#[facet(rename_all = "kebab-case")]
pub struct GlobalArgs {
    // r[impl cli.global.debug]
    /// Enable debug logging, including backtraces on panics.
    #[facet(args::named, default)]
    pub debug: bool,

    /// Log level filter directive.
    #[facet(args::named)]
    pub log_filter: Option<String>,

    /// Write structured ndjson logs.
    ///
    /// If a file path is provided, logs are written to that file.
    /// If a directory path is provided, a filename like `log_<timestamp>.ndjson`
    /// is generated in that directory.
    /// If omitted, no JSON log file is written.
    // r[impl cli.global.log-file]
    #[facet(args::named)]
    pub log_file: Option<String>,

    /// Request graceful cancellation after a span, log message, or duration.
    #[facet(flatten, default)]
    #[arbitrary(default)]
    pub stop_after: StopAfterArgs,

    /// Render command output as `text`, `json`, or `csv`.
    #[facet(args::named)]
    #[arbitrary(default)]
    pub output_format: Option<OutputFormat>,
}
