use super::types::User;
use spinners::{Spinner, Spinners};
use std::error::Error;
use std::process::Command;

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

pub fn convert_bytes(bytes: u64) -> String {
    let kb = bytes as f64 / 1024.0;
    let mb = kb / 1024.0;
    let gb = mb / 1024.0;
    let tb = gb / 1024.0;

    if tb >= 1.0 {
        format!("{:.2} Tb", tb)
    } else if gb >= 1.0 {
        format!("{:.2} Gb", gb)
    } else if mb >= 1.0 {
        format!("{:.2} Mb", mb)
    } else {
        format!("{:.2} Kb", kb)
    }
}

pub fn is_default<T: Default + PartialEq>(t: &T) -> bool {
    t == &T::default()
}
