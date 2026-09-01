use anyhow::Result;

use crate::update;

pub fn run() -> Result<()> {
    let current = env!("CARGO_PKG_VERSION");
    match update::latest_tag()? {
        Some(tag) if tag.trim_start_matches('v') != current => {
            println!("A newer release is available: {tag} (this is {current})");
            println!("Install it with:");
            println!("  {}", update::install_command());
        }
        Some(_) => println!("{} {current} is the latest release.", env!("CARGO_PKG_NAME")),
        None => println!("Cannot tell what the latest release is right now."),
    }
    Ok(())
}
