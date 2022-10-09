use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct Config {
    pub default_category: Option<String>,
    pub release_pr_label: Option<String>,
    pub label_categories: HashMap<String, String>,
    pub template: String,
}
