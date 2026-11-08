use std::path::{Path, PathBuf};

use serde::Serialize;
use surt_engine::project::Project;
use tauri::State;
use tokio::process::Command;

use crate::drivers::Drivers;
use crate::workspace::{Failure, Workspace};

const BRANCH_PREFIX: &str = "refs/heads/";
const SHORT_SHA: usize = 7;
const WORKTREES_SUFFIX: &str = "-worktrees";
const MANAGED_WORKTREES: [&str; 2] = [".surt", "worktrees"];
const FIELD_SEPARATOR: char = '\u{1f}';

#[derive(Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Checkouts {
    worktrees: Vec<WorktreeView>,
    branches: Vec<BranchView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeView {
    path: String,
    name: String,
    branch: Option<String>,
    head: String,
    main: bool,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchView {
    name: String,
    worktree: Option<String>,
}

#[tauri::command]
pub async fn checkouts(workspace: State<'_, Workspace>) -> Result<Checkouts, Failure> {
    let project = workspace.project().await?;
    read(&project).await
}

#[tauri::command]
pub async fn switch_branch(
    worktree: String,
    branch: String,
    create: bool,
    workspace: State<'_, Workspace>,
    drivers: State<'_, Drivers>,
) -> Result<Checkouts, Failure> {
    let project = workspace.project().await?;
    let known = read(&project).await?;
    let target = known
        .worktrees
        .iter()
        .find(|candidate| candidate.path == worktree)
        .ok_or_else(|| Failure::UnknownWorktree(worktree.clone()))?;
    if target.main && drivers.is_driving() {
        return Err(Failure::DrivingInMain);
    }
    let branch = branch.trim();
    let mut args = vec!["switch"];
    if create {
        args.push("-c");
    }
    args.push(branch);
    git(Path::new(&target.path), &args).await?;
    read(&project).await
}

#[tauri::command]
pub async fn create_worktree(
    branch: String,
    base: Option<String>,
    workspace: State<'_, Workspace>,
) -> Result<Checkouts, Failure> {
    let project = workspace.project().await?;
    let root = project.root();
    let branch = branch.trim().to_owned();
    let path = worktree_path(root, &branch).ok_or_else(|| Failure::NoSlug(branch.clone()))?;
    let path_text = path.to_string_lossy().into_owned();
    let exists = git(
        root,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{BRANCH_PREFIX}{branch}"),
        ],
    )
    .await
    .is_ok();
    if exists {
        git(root, &["worktree", "add", &path_text, &branch]).await?;
    } else {
        let base = base.unwrap_or_else(|| "HEAD".to_owned());
        git(root, &["worktree", "add", "-b", &branch, &path_text, &base]).await?;
    }
    read(&project).await
}

async fn read(project: &Project) -> Result<Checkouts, Failure> {
    let root = project.root();
    let porcelain = git(root, &["worktree", "list", "--porcelain"]).await?;
    let managed = managed_root();
    let worktrees = parse_worktrees(&porcelain, managed.as_deref());
    let format = format!("--format=%(refname:short){FIELD_SEPARATOR}%(worktreepath)");
    let refs = git(
        root,
        &[
            "for-each-ref",
            "--sort=-committerdate",
            &format,
            BRANCH_PREFIX,
        ],
    )
    .await?;
    let shown: Vec<&str> = worktrees
        .iter()
        .map(|worktree| worktree.path.as_str())
        .collect();
    Ok(Checkouts {
        branches: parse_branches(&refs, &shown),
        worktrees,
    })
}

fn managed_root() -> Option<PathBuf> {
    let home = std::env::home_dir()?;
    let home = canonical_home(&home);
    Some(
        MANAGED_WORKTREES
            .iter()
            .fold(home, |path, part| path.join(part)),
    )
}

fn canonical_home(path: &Path) -> PathBuf {
    std::fs::canonicalize(path)
        .map(|canonical| PathBuf::from(normalize(&canonical.to_string_lossy())))
        .unwrap_or_else(|_| path.to_path_buf())
}

fn normalize(path: &str) -> String {
    path.trim_start_matches(r"\\?\").replace('\\', "/")
}

pub fn parse_worktrees(porcelain: &str, managed: Option<&Path>) -> Vec<WorktreeView> {
    let managed = managed.map(|path| normalize(&path.to_string_lossy()).to_lowercase());
    let mut worktrees = Vec::new();
    for (index, block) in porcelain
        .split("\n\n")
        .filter(|block| !block.trim().is_empty())
        .enumerate()
    {
        let mut path = None;
        let mut head = String::new();
        let mut branch = None;
        let mut skip = false;
        for line in block.lines() {
            if let Some(value) = line.strip_prefix("worktree ") {
                path = Some(normalize(value));
            } else if let Some(value) = line.strip_prefix("HEAD ") {
                head = value.chars().take(SHORT_SHA).collect();
            } else if let Some(value) = line.strip_prefix("branch ") {
                branch = Some(value.trim_start_matches(BRANCH_PREFIX).to_owned());
            } else if line == "bare" || line.starts_with("prunable") {
                skip = true;
            }
        }
        let Some(path) = path else { continue };
        let is_managed = managed
            .as_deref()
            .is_some_and(|root| path.to_lowercase().starts_with(root));
        if skip || is_managed {
            continue;
        }
        let name = path.rsplit('/').next().unwrap_or(&path).to_owned();
        worktrees.push(WorktreeView {
            name,
            path,
            branch,
            head,
            main: index == 0,
        });
    }
    worktrees
}

pub fn parse_branches(refs: &str, shown: &[&str]) -> Vec<BranchView> {
    refs.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let (name, worktree) = line.split_once(FIELD_SEPARATOR).unwrap_or((line, ""));
            let worktree = normalize(worktree.trim());
            BranchView {
                name: name.to_owned(),
                worktree: shown
                    .iter()
                    .find(|path| path.eq_ignore_ascii_case(&worktree))
                    .map(|path| (*path).to_owned()),
            }
        })
        .collect()
}

pub fn worktree_path(root: &Path, branch: &str) -> Option<PathBuf> {
    let slug: String = branch
        .rsplit('/')
        .next()?
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric()
                || character == '-'
                || character == '_'
                || character == '.'
            {
                character
            } else {
                '-'
            }
        })
        .collect();
    let slug = slug.trim_matches(|character| character == '-' || character == '.');
    if slug.is_empty() {
        return None;
    }
    let repo = root.file_name()?.to_string_lossy().into_owned();
    Some(
        root.parent()?
            .join(format!("{repo}{WORKTREES_SUFFIX}"))
            .join(slug),
    )
}

async fn git(dir: &Path, args: &[&str]) -> Result<String, Failure> {
    let mut command = Command::new("git");
    command.arg("-C").arg(dir).args(args);
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    let output = command
        .output()
        .await
        .map_err(|error| Failure::Git(error.to_string()))?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
    }
    let message = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    Err(Failure::Git(if message.is_empty() {
        format!("git {} failed", args.join(" "))
    } else {
        message
    }))
}

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[cfg(test)]
mod tests;
