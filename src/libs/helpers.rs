use std::collections::{BTreeMap, HashMap};

use serde::{Serialize, Serializer};

pub fn to_hash_map(value: &[(&str, &str)]) -> HashMap<String, String> {
    return value.into_iter().map(|(p, v)| (p.to_string(), v.to_string())).collect::<HashMap<_, _>>();
}

pub fn ordered_map<S, K: Ord + Serialize, V: Serialize>(value: &HashMap<K, V>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let ordered: BTreeMap<_, _> = value.iter().collect();
    ordered.serialize(serializer)
}
