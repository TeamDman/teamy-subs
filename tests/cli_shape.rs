use std::process::Command;

#[test]
fn download_commands_live_under_provider_groups() {
    let output = Command::new(env!("CARGO_BIN_EXE_teamy-subs"))
        .args(["help", "list", "--short"])
        .output()
        .expect("failed to inspect CLI command list");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let commands = stdout.lines().collect::<Vec<_>>();
    assert!(
        commands
            .iter()
            .any(|line| line.ends_with(" yt-dlp download"))
    );
    assert!(
        commands
            .iter()
            .any(|line| line.ends_with(" subdl subtitle search"))
    );
    assert!(
        commands
            .iter()
            .any(|line| line.ends_with(" subdl subtitle download"))
    );
    for command in ["login", "status", "logout"] {
        assert!(
            commands
                .iter()
                .any(|line| line.ends_with(&format!(" subdl {command}")))
        );
    }
    assert!(!commands.iter().any(|line| line.ends_with(".exe download")));
}
