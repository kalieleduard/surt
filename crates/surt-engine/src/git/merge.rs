use std::ffi::OsStr;
use std::path::Path;

use super::{GitError, failed, output, run, run_with, unexpected};

const CONFLICTED: i32 = 1;
const AUTHOR_DATE: &str = "GIT_AUTHOR_DATE";
const COMMITTER_DATE: &str = "GIT_COMMITTER_DATE";

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum MergeTree {
    Clean { tree: String },
    Conflicted { paths: Vec<String> },
}

pub(crate) async fn merge_tree(
    cwd: &Path,
    ours: &str,
    theirs: &str,
) -> Result<MergeTree, GitError> {
    let args = [
        "merge-tree",
        "--write-tree",
        "-z",
        "--name-only",
        "--no-messages",
        ours,
        theirs,
    ];
    let output = output(cwd, &args, &[]).await?;
    let listing = String::from_utf8_lossy(&output.stdout).into_owned();
    let mut fields = listing.split('\0');
    let tree = fields.next().filter(|tree| !tree.is_empty());
    match (output.status.code(), tree) {
        (Some(0), Some(tree)) => Ok(MergeTree::Clean {
            tree: tree.to_owned(),
        }),
        (Some(CONFLICTED), Some(_)) => {
            let mut paths: Vec<String> = Vec::new();
            for path in fields.filter(|path| !path.is_empty()) {
                if !paths.iter().any(|known| known == path) {
                    paths.push(path.to_owned());
                }
            }
            Ok(MergeTree::Conflicted { paths })
        }
        (Some(0 | CONFLICTED), None) => Err(unexpected(&args, listing)),
        _ => Err(failed(&args, &output)),
    }
}

pub(crate) struct CommitTree<'a> {
    pub(crate) tree: &'a str,
    pub(crate) parents: [&'a str; 2],
    pub(crate) message: &'a str,
    pub(crate) unix_time: i64,
}

pub(crate) async fn commit_tree(cwd: &Path, commit: &CommitTree<'_>) -> Result<String, GitError> {
    let [first, second] = commit.parents;
    let date = format!("@{} +0000", commit.unix_time);
    let date = OsStr::new(&date);
    let args = [
        "commit-tree",
        "--no-gpg-sign",
        commit.tree,
        "-p",
        first,
        "-p",
        second,
        "-m",
        commit.message,
    ];
    let sha = run_with(cwd, &args, &[(AUTHOR_DATE, date), (COMMITTER_DATE, date)]).await?;
    Ok(sha.trim().to_owned())
}

pub(crate) async fn move_ref(
    cwd: &Path,
    reference: &str,
    new: &str,
    expected: &str,
) -> Result<(), GitError> {
    run(cwd, &["update-ref", "--no-deref", reference, new, expected]).await?;
    Ok(())
}
