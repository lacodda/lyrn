//! What the registries and GitHub hold now: the newest version of a crate or
//! an npm package, and the release GitHub calls the latest.
//!
//! Each is asked the way that has no quota to spend. crates.io through its
//! sparse index - the files cargo itself reads - rather than its API, which
//! asks crawlers for a second between questions; npm through its registry
//! document; GitHub through the redirect of `/releases/latest`, which the
//! line's installers read for the same reason. The bases come from the
//! environment only for the tests, which stand local servers in.

use std::time::Duration;

use reqwest::blocking::Client;

fn crates_index() -> String {
    std::env::var("LYRN_CRATES_INDEX").unwrap_or_else(|_| "https://index.crates.io".to_string())
}

fn npm_registry() -> String {
    std::env::var("LYRN_NPM_REGISTRY").unwrap_or_else(|_| "https://registry.npmjs.org".to_string())
}

fn github() -> String {
    std::env::var("LYRN_GITHUB_GIT").unwrap_or_else(|_| "https://github.com".to_string())
}

fn client(redirects: bool) -> Result<Client, String> {
    let mut builder = Client::builder()
        .user_agent(concat!("lyrn/", env!("CARGO_PKG_VERSION"), " (https://lyrn.lacodda.com)"))
        .timeout(Duration::from_secs(20));
    if !redirects {
        builder = builder.redirect(reqwest::redirect::Policy::none());
    }
    builder.build().map_err(|e| e.to_string())
}

/// The body at `url`; `None` when there is nothing there.
fn get(url: &str, accept: &str) -> Result<Option<String>, String> {
    let response = client(true)?.get(url).header("Accept", accept).send().map_err(|e| short(&e))?;
    match response.status().as_u16() {
        200 => response.text().map(Some).map_err(|e| short(&e)),
        404 | 410 => Ok(None),
        status => Err(format!("answered {status}")),
    }
}

/// A transport error in one line: reqwest's own runs to a paragraph.
fn short(error: &reqwest::Error) -> String {
    if error.is_timeout() {
        "no answer in time".to_string()
    } else if error.is_connect() {
        "could not connect".to_string()
    } else {
        error.to_string().lines().next().unwrap_or_default().to_string()
    }
}

/// Where the sparse index keeps a crate: `1/a`, `2/ab`, `3/a/abc`,
/// `se/rd/serde`.
fn index_path(name: &str) -> String {
    let name = name.to_ascii_lowercase();
    match name.len() {
        1 => format!("1/{name}"),
        2 => format!("2/{name}"),
        3 => format!("3/{}/{name}", &name[..1]),
        _ => format!("{}/{}/{name}", &name[..2], &name[2..4]),
    }
}

/// The newest version of a crate that is neither yanked nor a pre-release;
/// `None` when crates.io has never seen the crate.
pub fn newest_crate(name: &str) -> Result<Option<String>, String> {
    let Some(text) = get(&format!("{}/{}", crates_index().trim_end_matches('/'), index_path(name)), "text/plain")? else {
        return Ok(None);
    };
    Ok(newest_in_index(&text))
}

fn newest_in_index(text: &str) -> Option<String> {
    text.lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|entry| entry["yanked"].as_bool() != Some(true))
        .filter_map(|entry| entry["vers"].as_str().map(str::to_string))
        .filter(|v| !v.contains('-'))
        .max_by(|a, b| version_cmp(a, b))
}

/// The version npm installs by default; `None` when the registry has no
/// such package.
pub fn npm_latest(name: &str) -> Result<Option<String>, String> {
    // A scoped name keeps its `@` and has its slash escaped.
    let path = name.replacen('/', "%2f", 1);
    let Some(text) = get(
        &format!("{}/{path}", npm_registry().trim_end_matches('/')),
        "application/vnd.npm.install-v1+json",
    )?
    else {
        return Ok(None);
    };
    let document: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    Ok(document["dist-tags"]["latest"].as_str().map(str::to_string))
}

/// The tag GitHub calls the latest release of `owner/name`; `None` when it
/// has none.
pub fn github_latest(slug: &str) -> Result<Option<String>, String> {
    let url = format!("{}/{slug}/releases/latest", github().trim_end_matches('/'));
    let response = client(false)?.get(&url).send().map_err(|e| short(&e))?;
    if !response.status().is_redirection() {
        return match response.status().as_u16() {
            404 => Ok(None),
            status => Err(format!("answered {status}")),
        };
    }
    let location = response.headers().get("location").and_then(|l| l.to_str().ok()).unwrap_or_default();
    // A repository with no release redirects to its list of releases.
    Ok(location.rsplit_once("/releases/tag/").map(|(_, tag)| tag.to_string()))
}

/// Versions by their numbers, a pre-release before its release:
/// `10.9.0` < `10.34.5`, `2.0.0-rc.1` < `2.0.0`.
pub fn version_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let split = |v: &str| -> (Vec<u64>, bool) {
        let v = v.trim_start_matches('v');
        let (core, pre) = v.split_once('-').map_or((v, false), |(c, _)| (c, true));
        let core = core.split('+').next().unwrap_or(core);
        (core.split('.').map(|p| p.parse().unwrap_or(0)).collect(), pre)
    };
    let (a_core, a_pre) = split(a);
    let (b_core, b_pre) = split(b);
    a_core.cmp(&b_core).then(b_pre.cmp(&a_pre))
}

/// Whether moving from `from` to `to` crosses a breaking change, by Cargo's
/// reading of semver: the first number that is not zero changed.
pub fn breaking(from: &str, to: &str) -> bool {
    let parts = |v: &str| -> Vec<u64> {
        v.trim_start_matches('v')
            .split(['-', '+'])
            .next()
            .unwrap_or_default()
            .split('.')
            .map(|p| p.parse().unwrap_or(0))
            .collect()
    };
    let (a, b) = (parts(from), parts(to));
    let leading = |v: &[u64]| v.iter().position(|n| *n != 0).unwrap_or(v.len().saturating_sub(1));
    let at = leading(&a);
    a.get(..=at) != b.get(..=at)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_index_keeps_a_crate_where_cargo_looks() {
        assert_eq!(index_path("a"), "1/a");
        assert_eq!(index_path("ab"), "2/ab");
        assert_eq!(index_path("abc"), "3/a/abc");
        assert_eq!(index_path("Serde"), "se/rd/serde");
    }

    #[test]
    fn the_newest_is_neither_yanked_nor_a_pre_release() {
        let index = r#"{"name":"x","vers":"0.9.0","yanked":false}
{"name":"x","vers":"0.10.0","yanked":false}
{"name":"x","vers":"0.11.0","yanked":true}
{"name":"x","vers":"0.12.0-beta.1","yanked":false}"#;
        assert_eq!(newest_in_index(index).as_deref(), Some("0.10.0"));
    }

    #[test]
    fn versions_compare_by_number() {
        assert!(version_cmp("10.9.0", "10.34.5").is_lt());
        assert!(version_cmp("2.0.0-rc.1", "2.0.0").is_lt());
        assert!(version_cmp("v2.12.0", "2.12.0").is_eq());
    }

    #[test]
    fn a_break_is_where_cargo_sees_one() {
        assert!(breaking("0.13.5", "0.14.0"));
        assert!(!breaking("0.13.5", "0.13.9"));
        assert!(breaking("1.9.0", "2.0.0"));
        assert!(!breaking("1.2.0", "1.9.3"));
        assert!(breaking("0.0.3", "0.0.4"));
    }
}
