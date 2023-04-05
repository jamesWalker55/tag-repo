use crate::repo::{OpenError, Repo};
use std::path::Path;

struct RepoManager {
    repo: Repo,
}

#[derive(Debug)]
enum ConnectResponse {
    Open,
}

impl RepoManager {
    fn new(path: impl AsRef<Path>) -> Result<Self, OpenError> {
        let repo = Repo::open(path)?;
        Ok(Self { repo })
    }
}
