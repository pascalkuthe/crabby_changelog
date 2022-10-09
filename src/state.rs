use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};

mod change;

pub use change::{Change, ChangeMeta};

#[derive(Serialize, Deserialize, Clone, Hash, PartialEq, Eq)]
pub struct PullRequest {
    pub number: u64,
    pub url: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ReleaseState {
    #[serde(with = "change")]
    pub changes: IndexMap<Change, ChangeMeta>,
    pub authors: IndexSet<String>,
}

impl ReleaseState {
    pub fn insert_pr_change(&mut self, change: Change, pr: PullRequest) {
        self.changes.entry(change).or_default().insert(pr);
    }
}
