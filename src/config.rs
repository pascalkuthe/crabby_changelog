use indexmap::IndexSet;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize)]
pub struct Config {
    pub main_branch: String,
    #[serde(default)]
    pub changelog_branch: String,
    pub repo: String,
    pub default_group: Option<String>,
    pub release_pr_label: Option<String>,
    #[serde(default)]
    pub label_groups: HashMap<String, String>,
    pub template: String,
    #[serde(default)]
    pub groups: IndexSet<String>,
    #[serde(default)]
    pub ignored_labels: IndexSet<String>,
    #[serde(default)]
    pub ignored_authors: IndexSet<String>,
    #[serde(default)]
    pub ignored_title_prefix: IndexSet<String>,
}
