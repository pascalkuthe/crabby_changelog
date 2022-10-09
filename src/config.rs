use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize)]
pub struct Config {
    pub main_branch: String,
    pub changelog_branch: String,
    pub repo: String,
    pub default_group: Option<String>,
    pub release_pr_label: Option<String>,
    pub label_groups: HashMap<String, String>,
    pub template: String,
    pub groups: Vec<String>,
}
