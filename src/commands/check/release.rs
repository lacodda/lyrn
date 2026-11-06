//! What furca's release engine reads, and what a release put out.
//!
//! `release` holds `release.toml` - the file the engine releases a repository
//! from - to the engine's own reading of it, on every commit. `installers`
//! asks the engine, too, whether every target an installer asks for is a file
//! the release page will carry. With `--release`, the checks of a release
//! that went out: its page and the files `[page] assets` lists, the
//! registries, the links the manifests show, the `Checked live` list its
//! notes end with, and with `--install` the build its installer puts in
//! place.
//!
//! Every check here but `live` is the engine's own, read through the
//! `furca-release` crate rather than written again: `furca release verify`
//! and `lyrn check --release` ask the same questions the same way, and a
//! release that passes one passes the other.

use std::path::Path;
use std::time::Duration;

use furca_release::hosting::{self, Forge, Place};
use furca_release::{Check, Config, Error, Release, Shops, Sources, Status};

use super::Shop;
use crate::commands::doctor::{Finding, Outcome, Repo};

/// The file the engine reads, at the root.
const FILE: &str = furca_release::FILE;

/// The heading of the list every release's notes end with.
const LIVE: &str = "### Checked live";

/// `release`: the repository says how it is released, in a file the engine
/// reads without a problem.
pub fn check(repo: &Repo) -> Outcome {
    if !repo.has_file(FILE) {
        let releases = [".github/workflows/release.yml", ".github/workflows/publish.yml"]
            .into_iter()
            .find(|workflow| repo.has_file(workflow));
        return match releases {
            Some(workflow) => Outcome::Found(vec![
                Finding::warn(format!("{workflow} puts releases out, and no {FILE} says how furca releases them")).fix(format!(
                    "write {FILE} - `lyrn new` writes one for every form; its keys: https://lacodda.github.io/furca/reference/release/"
                )),
            ]),
            None => Outcome::Skip(format!("no {FILE}, and no workflow that releases")),
        };
    }
    match Config::load(&repo.root) {
        Ok(config) => {
            let packages = config.package.len();
            let what = match packages {
                0 => "nothing for a registry".to_string(),
                1 => "1 package".to_string(),
                n => format!("{n} packages"),
            };
            Outcome::Pass(format!("furca reads {FILE}: {}, {what}", plural(config.project.manifests.len(), "manifest")))
        }
        Err(Error::Config(problems)) => Outcome::Found(
            problems
                .into_iter()
                .map(|problem| Finding::fail(format!("{FILE}: {problem}")).fix("the keys furca reads: https://lacodda.github.io/furca/reference/release/"))
                .collect(),
        ),
        Err(other) => Outcome::Found(vec![Finding::fail(format!("furca cannot read {FILE}: {other}"))]),
    }
}

/// The engine's own `installers` check: every target the installers ask for
/// is among the files `[page] assets` lists. `None` when there is no
/// `release.toml` to hold them to, or no installer.
pub fn installers(repo: &Repo) -> Option<Vec<Finding>> {
    // The engine reads a work tree: the installers it holds are the tracked
    // ones. A release.toml that does not load is `release`'s to report.
    if repo.git.is_none() || !repo.has_file(FILE) || Config::load(&repo.root).is_err() {
        return None;
    }
    let release = match Release::open(&repo.root) {
        Ok(release) => release,
        Err(error) => {
            return Some(vec![Finding::warn(format!(
                "furca cannot open the repository to hold its installers to {FILE}: {error}"
            ))]);
        }
    };
    let sources = Sources {
        registries: None,
        record: None,
        now: std::time::SystemTime::now(),
    };
    let plan = match release.plan(&sources) {
        Ok(plan) => plan,
        Err(error) => return Some(vec![Finding::warn(format!("furca cannot hold the installers to {FILE}: {error}"))]),
    };
    let check = plan.checks.iter().find(|c| c.name == "installers")?;
    Some(findings(check))
}

/// The checks of the release `tag`, or of the newest release: the page, the
/// registries, the links, the `Checked live` list, and with `install` the
/// installed build.
pub fn released(root: &Path, tag: Option<&str>, install: bool) -> Vec<(Shop, Outcome)> {
    let on_every = |outcome: Outcome| {
        let mut shops = vec![Shop::Page, Shop::Registries, Shop::Links, Shop::Live];
        if install {
            shops.push(Shop::Install);
        }
        shops.into_iter().map(|shop| (shop, outcome.clone())).collect::<Vec<_>>()
    };
    if !root.join(FILE).is_file() {
        return on_every(Outcome::Skip(format!("no {FILE} to read the release from")));
    }
    let release = match Release::open(root) {
        Ok(release) => release,
        Err(error) => return on_every(Outcome::Found(vec![Finding::fail(format!("furca cannot read the release: {error}"))])),
    };
    let tag = match tag {
        Some(tag) => tag.to_string(),
        None => match release.released_tag() {
            Ok(Some(tag)) => tag,
            Ok(None) => return on_every(Outcome::Skip("no release yet: no vX.Y.Z tag".to_string())),
            Err(error) => return on_every(Outcome::Found(vec![Finding::fail(format!("the tags cannot be read: {error}"))])),
        },
    };
    let Some((place, forge)) = forge(&release) else {
        return on_every(Outcome::Found(vec![
            Finding::fail("no forge to ask: the branch follows no remote and there is no `origin` on GitHub or Forgejo")
                .fix("push the repository to its forge, then check again"),
        ]));
    };

    let http = furca_release::Http::default();
    let shops = Shops {
        registries: &http,
        web: &http,
        forge: forge.as_ref(),
    };
    let mut outcomes = match release.verify(&tag, &shops) {
        Ok(verified) => verified
            .checks
            .iter()
            .filter_map(|check| {
                let shop = match check.name {
                    "page" => Shop::Page,
                    "registries" => Shop::Registries,
                    "links" => Shop::Links,
                    _ => return None,
                };
                Some((shop, outcome(check, &tag)))
            })
            .collect(),
        Err(error) => vec![(Shop::Page, Outcome::Found(vec![Finding::fail(format!("{tag}: {error}"))]))],
    };
    outcomes.push((Shop::Live, live(&place, &tag)));

    if install {
        outcomes.push((Shop::Install, installed(&release, &tag, forge.as_ref())));
    }
    outcomes
}

/// The forge behind the remote the branch follows, or `origin`.
fn forge(release: &Release) -> Option<(Place, Box<dyn Forge>)> {
    let repo = release.repository();
    let url = match repo.upstream().ok().flatten() {
        Some(upstream) => upstream.url,
        None => repo.remote_url("origin").ok().flatten()?,
    };
    let place = Place::from_url(&url, Some(release.config().ci.provider))?;
    let forge = hosting::Http::new(place.clone());
    Some((place, Box::new(forge)))
}

/// `install`: the release installed by the repository's own installer into a
/// temporary directory - never the user's place, and never onto the PATH -
/// and proven to be the release's own build.
fn installed(release: &Release, tag: &str, forge: &dyn Forge) -> Outcome {
    if release.config().post.install.is_none() {
        return Outcome::Skip(format!("{FILE} has no [post.install]: nothing here installs from the release"));
    }
    let dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(error) => return Outcome::Found(vec![Finding::fail(format!("no temporary directory to install into: {error}"))]),
    };
    // The installer's own output goes to standard error: standard output
    // carries the report, or one JSON document.
    let machine = furca_release::Local {
        output: furca_release::Output::Stderr,
    };
    let mut say = |line: &str| eprintln!("  install  {line}");
    let checks = match release.install_check(tag, dir.path(), forge, &machine, &mut say) {
        Ok(checks) => checks,
        Err(error) => return Outcome::Found(vec![Finding::fail(format!("{tag}: {error}"))]),
    };
    let found: Vec<Finding> = checks.iter().flat_map(findings).collect();
    Outcome::from_findings(found, || {
        let build = checks.iter().find(|c| c.name == "build").map(|c| c.summary.clone());
        match build {
            Some(build) => format!("{tag} installed by its own installer: {build}"),
            None => format!("{tag} installed by its own installer"),
        }
    })
}

/// `live`: the `Checked live` list the release's notes end with, every box
/// ticked.
fn live(place: &Place, tag: &str) -> Outcome {
    let notes = match release_notes(place, tag) {
        Ok(Some(notes)) => notes,
        Ok(None) => return Outcome::Found(vec![Finding::fail(format!("{tag} has no release page to read the notes of"))]),
        Err(why) => return Outcome::Found(vec![Finding::warn(format!("the notes of {tag} cannot be read: {why}"))]),
    };
    let Some(list) = checklist(&notes) else {
        return Outcome::Skip(format!("the notes of {tag} end with no `{}` list", LIVE.trim_start_matches('#').trim()));
    };
    let open: Vec<Finding> = list
        .iter()
        .filter(|(ticked, _)| !ticked)
        .map(|(_, item)| {
            Finding::warn(format!("{tag}: not ticked yet - {item}")).fix(format!(
                "tick it on the release page once it holds: {}/{}/{}/releases/tag/{tag}",
                place.web, place.owner, place.repo
            ))
        })
        .collect();
    Outcome::from_findings(open, || format!("{tag}: every box of `Checked live` ticked ({})", list.len()))
}

/// The items of the `Checked live` list in `notes`, each with whether it is
/// ticked; `None` when the notes have no such list.
pub fn checklist(notes: &str) -> Option<Vec<(bool, String)>> {
    let mut lines = notes.lines().skip_while(|line| line.trim() != LIVE);
    lines.next()?;
    let items = lines
        .take_while(|line| !line.trim_start().starts_with('#'))
        .filter_map(|line| {
            let line = line.trim();
            if let Some(item) = line.strip_prefix("- [ ]") {
                return Some((false, item.trim().to_string()));
            }
            let item = line.strip_prefix("- [x]").or_else(|| line.strip_prefix("- [X]"))?;
            Some((true, item.trim().to_string()))
        })
        .collect();
    Some(items)
}

/// The notes of the release `tag`, as the forge's API keeps them; `None`
/// when the tag has no release page. Asked with a token when the
/// environment offers one, as furca asks.
fn release_notes(place: &Place, tag: &str) -> Result<Option<String>, String> {
    let api = match std::env::var("LYRN_FORGE_API") {
        Ok(base) => base,
        Err(_) if place.web == "https://github.com" => "https://api.github.com".to_string(),
        Err(_) if matches!(place.provider, furca_release::Provider::Github) => format!("{}/api/v3", place.web),
        Err(_) => format!("{}/api/v1", place.web),
    };
    let url = format!("{api}/repos/{}/{}/releases/tags/{tag}", place.owner, place.repo);
    let client = reqwest::blocking::Client::builder()
        .user_agent(concat!("lyrn/", env!("CARGO_PKG_VERSION"), " (https://lyrn.lacodda.com)"))
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())?;
    let mut request = client.get(&url).header("Accept", "application/json");
    if let Some(token) = token(place) {
        request = match place.provider {
            furca_release::Provider::Github => request.bearer_auth(token),
            furca_release::Provider::Forgejo => request.header("Authorization", format!("token {token}")),
        };
    }
    let response = request.send().map_err(|e| e.to_string())?;
    match response.status().as_u16() {
        404 => Ok(None),
        403 | 429 => Err(format!(
            "{} refused the question ({}); give it a token in GH_TOKEN",
            place.web,
            response.status()
        )),
        200 => {
            let text = response.text().map_err(|e| e.to_string())?;
            let page: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
            Ok(Some(page.get("body").and_then(|b| b.as_str()).unwrap_or_default().to_string()))
        }
        status => Err(format!("{url} answered {status}")),
    }
}

/// The token the environment offers for a forge: GitHub's from `GH_TOKEN`,
/// `GITHUB_TOKEN` or the `gh` CLI, Forgejo's from `FORGEJO_TOKEN`.
fn token(place: &Place) -> Option<String> {
    let from_env = |names: &[&str]| names.iter().find_map(|n| std::env::var(n).ok().filter(|v| !v.trim().is_empty()));
    match place.provider {
        furca_release::Provider::Forgejo => from_env(&["FORGEJO_TOKEN"]),
        furca_release::Provider::Github => from_env(&["GH_TOKEN", "GITHUB_TOKEN"]).or_else(|| {
            if place.web != "https://github.com" {
                return None;
            }
            let output = std::process::Command::new("gh").args(["auth", "token"]).output().ok()?;
            let token = String::from_utf8(output.stdout).ok()?.trim().to_string();
            (output.status.success() && !token.is_empty()).then_some(token)
        }),
    }
}

/// One of the engine's checks as an outcome of lyrn's.
fn outcome(check: &Check, tag: &str) -> Outcome {
    match check.status {
        Status::Pass => Outcome::Pass(format!("{tag}: {}", check.summary)),
        Status::Skip => Outcome::Skip(format!("{tag}: {}", check.summary)),
        Status::Warn | Status::Fail => Outcome::Found(findings(check)),
    }
}

/// What an engine's check found: nothing when it passed or did not apply.
fn findings(check: &Check) -> Vec<Finding> {
    let finding = match check.status {
        Status::Pass | Status::Skip => return Vec::new(),
        Status::Warn => Finding::warn(check.summary.clone()),
        Status::Fail => Finding::fail(check.summary.clone()),
    };
    if check.details.is_empty() {
        vec![finding]
    } else {
        vec![finding.fix(check.details.join("\n"))]
    }
}

fn plural(n: usize, word: &str) -> String {
    if n == 1 { format!("1 {word}") } else { format!("{n} {word}s") }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_checklist_is_read_from_its_heading_to_the_next() {
        let notes = "## [1.2.0] - 2026-11-01\n\n### Features\n\n- [ ] not this one\n\n### Checked live\n\n\
                     - [x] Installed from this release\n- [ ] Run by hand\n\nA closing line.\n\n### After\n\n- [ ] nor this\n";
        let list = checklist(notes).expect("the list is there");
        assert_eq!(
            list,
            vec![(true, "Installed from this release".to_string()), (false, "Run by hand".to_string())]
        );
    }

    #[test]
    fn notes_without_the_list_have_none() {
        assert_eq!(checklist("## [1.2.0]\n\n- [ ] a task of the changelog\n"), None);
    }

    #[test]
    fn a_capital_x_ticks_a_box_too() {
        let list = checklist("### Checked live\n- [X] done\n").unwrap();
        assert_eq!(list, vec![(true, "done".to_string())]);
    }
}
