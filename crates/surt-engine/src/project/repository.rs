use super::{Error, Project};
use crate::git;

const USER_VARIABLES: [&str; 2] = ["USER", "USERNAME"];
const UNKNOWN_USER: &str = "unknown";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Commit {
    pub sha: String,
    pub branch: Option<String>,
}

impl Project {
    pub async fn resolve_commit(&self, reference: &str) -> Result<Option<Commit>, Error> {
        let root = self.layout.root();
        let Some(sha) = git::commit_sha(root, reference).await? else {
            return Ok(None);
        };
        let branch = git::branch_name(root, reference).await?;
        Ok(Some(Commit { sha, branch }))
    }

    pub async fn actor(&self) -> Result<String, Error> {
        let email = git::committer_email(self.layout.root()).await?;
        Ok(email.unwrap_or_else(os_user))
    }
}

fn os_user() -> String {
    USER_VARIABLES
        .iter()
        .find_map(|variable| std::env::var(variable).ok())
        .unwrap_or_else(|| UNKNOWN_USER.to_owned())
}
