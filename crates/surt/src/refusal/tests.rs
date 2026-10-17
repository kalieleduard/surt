use std::path::PathBuf;

use surt_engine::project::{DefinitionFile, GitVersion, InvalidDefinition, Problem, Refusal};

use super::message;

const MAX_CONTEXT_LINES: usize = 2;
const MAX_WAYS_OUT: usize = 3;
const LABELS: [&str; 8] = [
    "fix:",
    "next:",
    "resume:",
    "regenerate:",
    "undo:",
    "inspect:",
    "or:",
    "run:",
];

fn text(refusal: &Refusal) -> String {
    message(refusal).plain_error()
}

fn every_refusal() -> Vec<Refusal> {
    let path = PathBuf::from("/work/app");
    vec![
        Refusal::NotAProject { root: path.clone() },
        Refusal::InsideManagedWorktree {
            path: path.clone(),
            project: Some(path.clone()),
        },
        Refusal::InsideManagedWorktree {
            path: path.clone(),
            project: None,
        },
        Refusal::NotAGitRepository { path: path.clone() },
        Refusal::BareRepository { path: path.clone() },
        Refusal::GitNotFound,
        Refusal::GitTooOld {
            found: GitVersion {
                major: 2,
                minor: 34,
                patch: 1,
            },
            min: GitVersion {
                major: 2,
                minor: 38,
                patch: 0,
            },
        },
        Refusal::NoGitIdentity,
        Refusal::SchemaTooNew {
            db: 7,
            supported: 5,
        },
        Refusal::InvalidConfig {
            path: path.join(".surt/config.toml"),
            line: Some(2),
            reason: "unknown field `max_atempts`".to_owned(),
        },
        Refusal::InvalidConfig {
            path: path.join(".surt/config.toml"),
            line: None,
            reason: "invalid TOML".to_owned(),
        },
        Refusal::MergeIntoBaseUnsupported {
            path: path.join(".surt/config.toml"),
        },
        Refusal::InvalidDefinition(Box::new(InvalidDefinition {
            file: DefinitionFile::Workflow("feature".to_owned()),
            step: Some("ship".to_owned()),
            problem: Problem::MissingKind,
        })),
        Refusal::InvalidDefinition(Box::new(InvalidDefinition {
            file: DefinitionFile::Agent("builder".to_owned()),
            step: None,
            problem: Problem::MissingClaudeProfile,
        })),
    ]
}

#[test]
fn every_refusal_has_one_headline_at_most_two_context_lines_and_one_to_three_ways_out() {
    for refusal in every_refusal() {
        let text = text(&refusal);
        let mut lines = text.lines();
        let headline = lines.next().expect("a headline");
        let rest: Vec<&str> = lines.collect();
        let ways = rest
            .iter()
            .filter(|line| {
                LABELS
                    .iter()
                    .any(|label| line.trim_start().starts_with(label))
            })
            .count();

        assert!(headline.starts_with("✖ "), "{text}");
        assert!(rest.iter().all(|line| line.starts_with("  ")), "{text}");
        assert!((1..=MAX_WAYS_OUT).contains(&ways), "{text}");
        assert!(rest.len() - ways <= MAX_CONTEXT_LINES, "{text}");
    }
}

#[test]
fn an_old_git_names_the_found_and_the_required_version() {
    let refusal = &every_refusal()[6];

    assert!(
        text(refusal).starts_with("✖ git 2.34.1 is too old; surt needs git 2.38 or later\n"),
        "{}",
        text(refusal)
    );
}

#[test]
fn a_newer_database_names_both_schema_versions() {
    let refusal = Refusal::SchemaTooNew {
        db: 7,
        supported: 5,
    };

    assert!(
        text(&refusal).starts_with("✖ .surt/surt.db is at schema 7; this surt supports up to 5\n")
    );
}

#[test]
fn a_managed_worktree_points_back_at_the_project_when_it_is_known() {
    let refusal = Refusal::InsideManagedWorktree {
        path: PathBuf::from("/home/you/.surt/worktrees/app-a1b2c3/20261017-add-login/A"),
        project: Some(PathBuf::from("/work/app")),
    };

    assert!(
        text(&refusal).contains("\n  fix: cd /work/app\n"),
        "{}",
        text(&refusal)
    );
}

#[test]
fn an_invalid_definition_names_the_file_and_the_step() {
    let refusal = &every_refusal()[12];

    assert!(
        text(refusal).starts_with("✖ invalid built-in definition: workflow feature [steps.ship]\n"),
        "{}",
        text(refusal)
    );
}
