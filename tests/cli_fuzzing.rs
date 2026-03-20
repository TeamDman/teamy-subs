//! CLI fuzzing tests using figue's arbitrary helper assertions.

use teamy_subs::cli::Cli;

#[test]
// r[verify cli.command.download]
// r[verify cli.command.extract]
// r[verify cli.command.convert]
// r[verify cli.global.debug]
// r[verify cli.global.log-file]
fn fuzz_cli_args_consistency() {
    figue::assert_to_args_consistency::<Cli>(5000)
        .expect("figue helper consistency check should pass");
}

#[test]
// r[verify cli.command.download]
// r[verify cli.command.extract]
// r[verify cli.command.convert]
// r[verify cli.global.debug]
// r[verify cli.global.log-file]
fn fuzz_cli_args_roundtrip() {
    figue::assert_to_args_roundtrip::<Cli>(500).expect("figue helper roundtrip check should pass");
}
