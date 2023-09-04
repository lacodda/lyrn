use super::types::User;
use serde::{Serialize, Serializer};
use spinners::{Spinner, Spinners};
use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::process::Command;

pub fn _ordered_map<S, K: Ord + Serialize, V: Serialize>(value: &HashMap<K, V>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let ordered: BTreeMap<_, _> = value.iter().collect();
    ordered.serialize(serializer)
}

pub fn _opt_ordered_map<S, K: Ord + Serialize, V: Serialize>(value: &Option<HashMap<K, V>>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let inside = value.as_ref().unwrap();
    let ordered: BTreeMap<_, _> = inside.iter().collect();
    ordered.serialize(serializer)
}

pub fn _to_hash_map(value: &[(&str, &str)]) -> HashMap<String, String> {
    return value.into_iter().map(|(p, v)| (p.to_string(), v.to_string())).collect::<HashMap<_, _>>();
}

pub fn _merge_opt_hashmaps<T>(a: Option<HashMap<String, T>>, b: Option<HashMap<String, T>>) -> Option<HashMap<String, T>> {
    if a.is_some() && b.is_some() {
        let mut base = a.unwrap();
        base.extend(b.unwrap().into_iter());
        return Some(base);
    }
    b.or(a)
}

pub fn _merge_opt_vectors<T>(a: Option<Vec<T>>, b: Option<Vec<T>>) -> Option<Vec<T>> {
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

pub fn clear_console() -> Result<(), Box<dyn Error>> {
    #[cfg(windows)]
    let mut clear_cmd = Command::new("cmd");
    #[cfg(windows)]
    clear_cmd.arg("/c").arg("cls");

    #[cfg(not(windows))]
    let mut clear_cmd: Command = Command::new("clear");

    clear_cmd.status().unwrap();
    Ok(())
}

pub fn spinner_start(msg: &str) -> Result<Spinner, Box<dyn Error>> {
    clear_console()?;
    Ok(Spinner::new(Spinners::Dots12, msg.into()))
}
