use std::mem::transmute;

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};

use crate::state::one_or_many::OneOrMany;

mod map_to_list;
mod one_or_many;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct Change {
    pub message: String,
    pub group: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
pub struct ChangeMeta<const PRETTY: bool> {
    pub pr: OneOrMany<u64, PRETTY>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ReleaseStateImpl<const PRETTY: bool> {
    #[serde(with = "map_to_list")]
    pub changes: IndexMap<Change, ChangeMeta<PRETTY>>,
    pub authors: IndexSet<String>,
}

pub type ReleaseState = ReleaseStateImpl<true>;

impl ReleaseState {
    pub fn insert_pr_change(&mut self, change: Change, pr: u64) {
        self.changes.entry(change).or_default().pr.0.insert(pr);
    }
    pub fn to_tera_ctx(&self) -> ReleaseStateImpl<false> {
        unsafe { transmute(self.clone()) }
    }
}
