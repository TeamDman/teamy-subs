use std::process::Command;

#[test]
fn version_output_includes_build_metadata() {
    let output = Command::new(env!("CARGO_BIN_EXE_teamy-subs"))
        .arg("--version")
        .output()
        .expect("failed to run --version");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("version output should be UTF-8");
    for expected in [
        env!("CARGO_PKG_VERSION"),
        "repo ",
        "branch ",
        "rev ",
        "worktree ",
        "built ",
    ] {
        assert!(
            stdout.contains(expected),
            "missing {expected} from version output"
        );
    }
}
