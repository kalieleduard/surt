use tokio::io::{AsyncBufReadExt, BufReader};

use super::{Script, spawn};

fn script(run: &[&str]) -> Script {
    Script {
        name: "probe".to_owned(),
        run: run.iter().map(|part| (*part).to_owned()).collect(),
        description: None,
    }
}

#[tokio::test]
async fn a_script_runs_in_the_project_root_and_streams_its_output() {
    let root = tempfile::TempDir::new().expect("a temporary directory");
    let mut process = spawn(&script(&["git", "--version"]), root.path()).expect("git starts");
    let (stdout, _) = process.take_output();
    let mut lines = BufReader::new(stdout.expect("piped stdout")).lines();

    let first = lines.next_line().await.expect("output is readable");

    assert!(first.is_some_and(|line| line.starts_with("git version")));
    assert_eq!(process.wait().await.expect("git ends"), 0);
}

#[tokio::test]
async fn a_missing_program_is_reported_when_spawning() {
    let root = tempfile::TempDir::new().expect("a temporary directory");

    assert!(spawn(&script(&["surt-no-such-program"]), root.path()).is_err());
}
