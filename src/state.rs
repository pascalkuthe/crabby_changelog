use std::collections::HashSet;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct PullRequest {
    pub number: u64,
    pub url: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Change {
    pub message: String,
    pub category: String,
    pub prs: Vec<PullRequest>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ReleaseState {
    pub changes: Vec<Change>,
    pub authors: HashSet<String>,
}
