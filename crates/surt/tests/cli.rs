use std::process::{Command, ExitStatus};

const USAGE_ERROR_EXIT_CODE: i32 = 2;

fn run_surt(args: &[&str]) -> ExitStatus {
    Command::new(env!("CARGO_BIN_EXE_surt"))
        .args(args)
        .output()
        .expect("the surt binary runs")
        .status
}

#[test]
fn unknown_command_exits_with_usage_error() {
    let status = run_surt(&["no-such-command"]);

    assert_eq!(status.code(), Some(USAGE_ERROR_EXIT_CODE));
}

#[test]
fn unknown_flag_exits_with_usage_error() {
    let status = run_surt(&["--no-such-flag"]);

    assert_eq!(status.code(), Some(USAGE_ERROR_EXIT_CODE));
}
