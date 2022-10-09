use indexmap::IndexSet;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use std::hash::Hash;

#[derive(Clone, PartialEq, Eq, Default)]
pub struct OneOrMany<T: PartialEq + Eq + Hash, const PRETTY: bool>(pub IndexSet<T>);

impl<T, const PRETTY: bool> Serialize for OneOrMany<T, PRETTY>
where
    T: Serialize + Hash + Eq,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.0.len() == 1 && PRETTY {
            self.0.get_index(0).unwrap().serialize(serializer)
        } else {
            self.0.serialize(serializer)
        }
    }
}

impl<'de, T, const PRETTY: bool> Deserialize<'de> for OneOrMany<T, PRETTY>
where
    T: Deserialize<'de> + Hash + Eq,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let vals = match OneOrManyImpl::<T>::deserialize(deserializer)? {
            OneOrManyImpl::One(val) => [val].into(),
            OneOrManyImpl::Many(vals) => vals,
        };
        Ok(Self(vals))
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum OneOrManyImpl<T: Eq + Hash> {
    /// Single value
    One(T),
    /// Array of values
    Many(IndexSet<T>),
}
