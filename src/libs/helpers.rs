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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clear_console() {
        let result = clear_console();
        assert!(result.is_ok(), "clear_console failed: {:?}", result);

        // Verify that the console is cleared
        #[cfg(windows)]
        let output = Command::new("cmd").arg("/c").arg("cls").output().expect("Failed to run 'cmd /c cls' command");
        #[cfg(not(windows))]
        let output = Command::new("clear").output().expect("Failed to run 'clear' command");

        assert!(output.status.success(), "Clearing the console failed");
    }

    #[test]
    fn test_convert_bytes_kb() {
        let bytes = 1024;
        let result = convert_bytes(bytes);
        assert_eq!(result, "1.00 Kb");
    }

    #[test]
    fn test_convert_bytes_mb() {
        let bytes = 1048576;
        let result = convert_bytes(bytes);
        assert_eq!(result, "1.00 Mb");
    }

    #[test]
    fn test_convert_bytes_gb() {
        let bytes = 1073741824;
        let result = convert_bytes(bytes);
        assert_eq!(result, "1.00 Gb");
    }

    #[test]
    fn test_convert_bytes_tb() {
        let bytes = 1099511627776;
        let result = convert_bytes(bytes);
        assert_eq!(result, "1.00 Tb");
    }

    #[test]
    fn test_convert_bytes_rounding() {
        let bytes = 1536;
        let result = convert_bytes(bytes);
        assert_eq!(result, "1.50 Kb");
    }

    #[test]
    fn test_is_default() {
        // Test with a type that implements Default and PartialEq
        let default_int: i32 = Default::default();
        assert!(is_default(&default_int));

        // Test with a custom type that implements Default and PartialEq
        #[derive(Default, PartialEq, Debug)]
        struct CustomType {
            value: i32,
        }
        let custom_default = CustomType::default();
        assert!(is_default(&custom_default));

        // Test with a non-default value of the custom type
        let custom_non_default = CustomType { value: 42 };
        assert!(!is_default(&custom_non_default));
    }
}
