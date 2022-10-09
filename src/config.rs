use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct Config {
    pub default_group: Option<String>,
    pub release_pr_label: Option<String>,
    pub label_groups: HashMap<String, String>,
    pub template: String,
    pub groups: Vec<String>,
}
