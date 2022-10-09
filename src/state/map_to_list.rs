use indexmap::IndexMap;
use serde::de::Deserializer;
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};

use std::hash::Hash;

#[derive(Serialize, Deserialize)]
pub struct FlatTuple<K, V> {
    #[serde(flatten)]
    key: K,
    #[serde(flatten)]
    val: V,
}

pub fn serialize<S, K, V>(map: &IndexMap<K, V>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    K: Serialize + Hash + Eq + Clone,
    V: Serialize + Clone,
{
    serializer.collect_seq(map.iter().map(|(key, val)| FlatTuple {
        key: key.clone(),
        val: val.clone(),
    }))
}

pub fn deserialize<'de, D, K, V>(deserializer: D) -> Result<IndexMap<K, V>, D::Error>
where
    D: Deserializer<'de>,
    K: Deserialize<'de> + Hash + Eq,
    V: Deserialize<'de>,
{
    let mut map = IndexMap::new();
    for change in Vec::<FlatTuple<K, V>>::deserialize(deserializer)? {
        map.insert(change.key, change.val);
    }
    Ok(map)
}
