//! Finding out whether a newer release exists.

use std::time::Duration;

use anyhow::{Context, Result};

const REPO: &str = "{{ repo }}";
const TIMEOUT: Duration = Duration::from_secs(10);

/// The tag of the latest release, read from the redirect `/releases/latest`
/// performs.
///
/// Deliberately not the REST API: unauthenticated calls to api.github.com are
/// capped at 60 per hour per address, so a check that used it would start
/// failing for reasons of somebody else's making.
pub fn latest_tag() -> Result<Option<String>> {
    let client = reqwest::blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(TIMEOUT)
        .user_agent(concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION")))
        .build()
        .context("building the HTTP client")?;

    let response = match client.get(format!("https://github.com/{REPO}/releases/latest")).send() {
        Ok(response) => response,
        // Being offline is not an error worth failing a command over.
        Err(_) => return Ok(None),
    };

    let Some(location) = response.headers().get(reqwest::header::LOCATION) else {
        return Ok(None);
    };
    let tag = location.to_str().ok().and_then(|l| l.rsplit('/').next()).map(str::to_string);

    // A repository with no releases redirects to the releases page itself.
    Ok(tag.filter(|t| t.starts_with('v')))
}

/// What to run to install the newer version.
pub fn install_command() -> String {
    if cfg!(windows) {
        format!("irm https://raw.githubusercontent.com/{REPO}/main/tools/install.ps1 | iex")
    } else {
        format!("curl -fsSL https://raw.githubusercontent.com/{REPO}/main/tools/install.sh | sh")
    }
}
