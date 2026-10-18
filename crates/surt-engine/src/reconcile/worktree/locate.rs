use std::fs;
use std::path::{Path, PathBuf};

use super::{PrepareError, Target, io_error};
use crate::git::{self, worktrees};
use crate::store::unix_now;

const HALF_CREATED_LOCK: &str = "initializing";
const GITDIR_PREFIX: &str = "gitdir:";
const ADMIN_DIRS: &str = "worktrees";
const ADMIN_INDEX: &str = "index";
const ORPHANED_SUFFIX: &str = ".orphaned-";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Located {
    Present,
    Added,
    Repaired(Repair),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Repair {
    HalfCreated,
    Missing,
    Orphaned { aside: PathBuf },
}

pub(super) async fn locate(target: &Target<'_>) -> Result<Located, PrepareError> {
    let entries = worktrees::list(target.repository).await?;
    let entry = entries
        .iter()
        .find(|entry| same_path(&entry.path, target.path));
    let Some(entry) = entry else {
        return add_where_free(target).await;
    };
    if !target.path.exists() {
        return replace(target, Repair::Missing).await;
    }
    if entry.lock_reason.as_deref() == Some(HALF_CREATED_LOCK) || !admin_index_exists(target.path) {
        return replace(target, Repair::HalfCreated).await;
    }
    worktrees::repair(target.repository, target.path).await?;
    Ok(Located::Present)
}

async fn add_where_free(target: &Target<'_>) -> Result<Located, PrepareError> {
    if !target.path.exists() || is_empty_dir(target.path) {
        add(target).await?;
        return Ok(Located::Added);
    }
    if !orphaned_from(target).await? {
        return Err(PrepareError::Foreign(target.path.to_path_buf()));
    }
    let aside = aside_path(target.path);
    fs::rename(target.path, &aside).map_err(io_error("move aside", target.path))?;
    add(target).await?;
    Ok(Located::Repaired(Repair::Orphaned { aside }))
}

async fn replace(target: &Target<'_>, repair: Repair) -> Result<Located, PrepareError> {
    worktrees::remove(target.repository, target.path).await?;
    add(target).await?;
    Ok(Located::Repaired(repair))
}

async fn add(target: &Target<'_>) -> Result<(), PrepareError> {
    let reason = format!("surt {}", target.owner);
    worktrees::add(
        target.repository,
        target.path,
        target.short_branch(),
        &reason,
    )
    .await?;
    Ok(())
}

async fn orphaned_from(target: &Target<'_>) -> Result<bool, PrepareError> {
    let Some(admin) = admin_dir(target.path) else {
        return Ok(false);
    };
    let common = git::common_dir(target.repository).await?;
    Ok(admin.starts_with(common.join(ADMIN_DIRS)) && !admin.exists())
}

fn admin_index_exists(path: &Path) -> bool {
    admin_dir(path).is_some_and(|admin| admin.join(ADMIN_INDEX).is_file())
}

fn admin_dir(path: &Path) -> Option<PathBuf> {
    let link = fs::read_to_string(path.join(".git")).ok()?;
    let admin = link.trim().strip_prefix(GITDIR_PREFIX)?.trim();
    Some(path.join(admin))
}

fn is_empty_dir(path: &Path) -> bool {
    fs::read_dir(path).is_ok_and(|mut entries| entries.next().is_none())
}

fn aside_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(format!("{ORPHANED_SUFFIX}{}", unix_now()));
    PathBuf::from(name)
}

pub(crate) fn same_path(listed: &Path, recorded: &Path) -> bool {
    if listed == recorded {
        return true;
    }
    match (dunce::canonicalize(listed), dunce::canonicalize(recorded)) {
        (Ok(listed), Ok(recorded)) => listed == recorded,
        _ => false,
    }
}
