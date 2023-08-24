use super::types::User;
use serde::{Serialize, Serializer};
use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::process::Command;

pub fn ordered_map<S, K: Ord + Serialize, V: Serialize>(value: &HashMap<K, V>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let ordered: BTreeMap<_, _> = value.iter().collect();
    ordered.serialize(serializer)
}

pub fn opt_ordered_map<S, K: Ord + Serialize, V: Serialize>(value: &Option<HashMap<K, V>>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let inside = value.as_ref().unwrap();
    let ordered: BTreeMap<_, _> = inside.iter().collect();
    ordered.serialize(serializer)
}

pub fn to_hash_map(value: &[(&str, &str)]) -> HashMap<String, String> {
    return value.into_iter().map(|(p, v)| (p.to_string(), v.to_string())).collect::<HashMap<_, _>>();
}

pub fn merge_opt_hashmaps<T>(a: Option<HashMap<String, T>>, b: Option<HashMap<String, T>>) -> Option<HashMap<String, T>> {
    if a.is_some() && b.is_some() {
        let mut base = a.unwrap();
        base.extend(b.unwrap().into_iter());
        return Some(base);
    }
    b.or(a)
}

pub fn merge_opt_vectors<T>(a: Option<Vec<T>>, b: Option<Vec<T>>) -> Option<Vec<T>> {
    if a.is_some() && b.is_some() {
        let mut base = a.unwrap();
        base.extend(b.unwrap());
        return Some(base);
    }
    b.or(a)
}

pub fn get_git_user() -> Result<User, Box<dyn Error>> {
    let user_name = Command::new("git").args(["config", "user.name"]).output()?;
    let user_email = Command::new("git").args(["config", "user.email"]).output()?;
    Ok(User {
        name: String::from_utf8_lossy(&user_name.stdout).trim().to_string(),
        email: String::from_utf8_lossy(&user_email.stdout).trim().to_string(),
    })
}
