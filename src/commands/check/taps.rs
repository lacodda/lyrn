//! `taps`: Homebrew and Scoop install the program.
//!
//! The line's programs go out through a tap and a bucket of their owner's -
//! `OWNER/homebrew-tap` and `OWNER/scoop-bucket` - whose formulas and
//! manifests are written from a `packages.json` by the newest release. A
//! program missing from that list is a README line that installs nothing:
//! `brew install OWNER/tap/NAME` answers that there is no such formula.

use serde_json::{Value, json};

use super::product::{Kind, Product};
use crate::commands::doctor::{Finding, Outcome, Repo};

/// The two lists, each with the words a person installs from it with.
const SHELVES: &[(&str, &str)] = &[("homebrew-tap", "brew install {owner}/tap/{name}"), ("scoop-bucket", "scoop install {name}")];

pub fn check(repo: &Repo, product: &Product) -> Outcome {
    if !installs_a_program(&repo.root) {
        return Outcome::Skip("release.toml has no [post.install]: no program here to install".to_string());
    }
    let Some(slug) = product.github.as_deref() else {
        return Outcome::Found(vec![Finding::fail("no GitHub repository is named here, so there is no owner's tap to look in")]);
    };
    let owner = slug.split('/').next().unwrap_or(slug);
    let entry = entry(repo, product, slug);
    let name = entry["name"].as_str().unwrap_or(&product.name).to_string();
    let lists = SHELVES
        .iter()
        .map(|(shelf, install)| {
            let list = format!("{owner}/{shelf}");
            let install = install.replace("{owner}", owner).replace("{name}", &name);
            (list.clone(), install, crate::registries::package_list(&list))
        })
        .collect();
    judge(product, slug, &entry, lists)
}

/// Whether release.toml says a program is installed from the release.
fn installs_a_program(root: &std::path::Path) -> bool {
    let Ok(text) = std::fs::read_to_string(root.join("release.toml")) else {
        return false;
    };
    toml::from_str::<toml::Table>(&text)
        .ok()
        .and_then(|t| t.get("post")?.get("install").cloned())
        .is_some()
}

/// What release.toml says the program is: the name `[post.install]` puts on
/// the PATH, and the name the archives `[page]` attaches begin with -
/// `efema-server-{tag}-<target>.tar.gz` - which is what the tap's and the
/// bucket's scripts look for.
fn released(root: &std::path::Path) -> (Option<String>, Option<String>) {
    let table = std::fs::read_to_string(root.join("release.toml"))
        .ok()
        .and_then(|t| toml::from_str::<toml::Table>(&t).ok());
    let Some(table) = table else { return (None, None) };
    let bin = table
        .get("post")
        .and_then(|p| p.get("install"))
        .and_then(|i| i.get("bin"))
        .and_then(|b| b.as_str())
        .map(str::to_string);
    let archives = table
        .get("page")
        .and_then(|p| p.get("assets"))
        .and_then(|a| a.as_array())
        .into_iter()
        .flatten()
        .filter_map(|a| a.as_str()?.split_once("-{tag}").map(|(prefix, _)| prefix.to_string()))
        .find(|prefix| !prefix.is_empty());
    (bin, archives)
}

/// The entry the lists should hold for this program.
pub fn entry(repo: &Repo, product: &Product, slug: &str) -> Value {
    let wanted = crate::commands::repo::Wanted::read(repo, product);
    let (installed, archives) = released(&repo.root);
    let program = installed
        .or_else(|| {
            product
                .manifests
                .iter()
                .filter(|m| m.kind == Kind::Cargo && m.publishable)
                .find_map(|m| m.bins.first().cloned())
        })
        .unwrap_or_else(|| product.name.clone());
    // The tap and the bucket install by the archives' name, which is the
    // program's where the product's own name is the workspace's.
    let name = archives.unwrap_or_else(|| program.clone());
    // The page of the crate whose program is installed, else the
    // workspace's; a library of the workspace has a page of its own.
    let workspace_homepage = repo
        .facts
        .as_ref()
        .ok()
        .and_then(|f| f.cargo.as_ref())
        .and_then(|c| c.get("workspace")?.get("package")?.get("homepage")?.as_str().map(str::to_string));
    let homepage = product
        .manifests
        .iter()
        .filter(|m| m.kind == Kind::Cargo && m.bins.contains(&program))
        .find_map(|m| m.homepage.clone())
        .or(workspace_homepage)
        .or(wanted.homepage)
        .unwrap_or_else(|| format!("https://github.com/{slug}"));
    let license = repo
        .facts
        .as_ref()
        .ok()
        .and_then(|f| f.cargo.as_ref())
        .and_then(|c| c.get("package")?.get("license")?.as_str().map(str::to_string))
        .unwrap_or_else(|| "MIT".to_string());
    // Homebrew's audit wants a short description with no full stop.
    let desc = wanted.description.unwrap_or_default().trim_end_matches('.').to_string();
    json!({
        "name": name,
        "repo": slug,
        "desc": desc,
        "homepage": homepage,
        "license": license,
        "bin": program,
    })
}

/// The lists as fetched, held to the entry. Each list is its `owner/repo`,
/// the command it installs with, and what fetching it gave.
pub fn judge(product: &Product, slug: &str, entry: &Value, lists: Vec<(String, String, Result<Option<String>, String>)>) -> Outcome {
    let mut findings = Vec::new();
    let mut found_in = Vec::new();
    let name = entry["name"].as_str().unwrap_or(&product.name);
    // In the order the lists keep their entries, rather than JSON's
    // alphabetical one.
    let fields: Vec<String> = ["name", "repo", "desc", "homepage", "license", "bin"]
        .iter()
        .map(|key| format!("  \"{key}\": {}", entry[*key]))
        .collect();
    let pretty = format!("{{\n{}\n}}", fields.join(",\n"));
    for (list, install, fetched) in lists {
        let text = match fetched {
            Err(reason) => {
                findings.push(Finding::fail(format!("{list}'s packages.json could not be read: {reason}")));
                continue;
            }
            Ok(None) => {
                findings.push(
                    Finding::fail(format!("{list} has no packages.json, so `{install}` finds nothing"))
                        .fix(format!("start {list} with a packages.json holding:\n{pretty}")),
                );
                continue;
            }
            Ok(Some(text)) => text,
        };
        let Ok(Value::Array(packages)) = serde_json::from_str::<Value>(&text) else {
            findings.push(Finding::fail(format!("{list}'s packages.json is not a list of packages")));
            continue;
        };
        let Some(package) = packages.iter().find(|p| p["repo"].as_str().is_some_and(|r| r.eq_ignore_ascii_case(slug))) else {
            findings.push(
                Finding::fail(format!("{name} is not in {list}'s packages.json, so `{install}` finds nothing"))
                    .fix(format!("add to packages.json in {list}:\n{pretty}")),
            );
            continue;
        };
        for (key, why) in [("name", "the name it is installed by"), ("bin", "the program it puts on the PATH")] {
            if package[key] != entry[key] {
                findings.push(
                    Finding::fail(format!(
                        "{list} lists `{key}` {} for {slug}, and the program's is {}: {why}",
                        package[key], entry[key]
                    ))
                    .fix(format!("`\"{key}\": {}` in {list}'s packages.json", entry[key])),
                );
            }
        }
        let homepage = |v: &Value| v.as_str().map(|s| s.trim_end_matches('/').to_string());
        if homepage(&package["homepage"]) != homepage(&entry["homepage"]) {
            findings.push(
                Finding::warn(format!(
                    "{list} sends people to {} for {slug}, and the manifests to {}",
                    package["homepage"], entry["homepage"]
                ))
                .fix(format!("`\"homepage\": {}` in {list}'s packages.json", entry["homepage"])),
            );
        }
        found_in.push(list);
    }
    if findings.is_empty() {
        Outcome::Pass(format!("{name} is in {}", found_in.join(" and ")))
    } else {
        Outcome::Found(findings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn product() -> Product {
        Product {
            name: "demo".into(),
            manifests: Vec::new(),
            github: Some("owner/demo".into()),
        }
    }

    fn entry() -> Value {
        json!({ "name": "demo", "repo": "owner/demo", "desc": "A demo", "homepage": "https://owner.github.io/demo/", "license": "MIT", "bin": "demo" })
    }

    fn lists(tap: Result<Option<String>, String>, bucket: Result<Option<String>, String>) -> Vec<(String, String, Result<Option<String>, String>)> {
        vec![
            ("owner/homebrew-tap".into(), "brew install owner/tap/demo".into(), tap),
            ("owner/scoop-bucket".into(), "scoop install demo".into(), bucket),
        ]
    }

    /// A workspace named after the product whose program is another crate's:
    /// the tap installs by the archives' name, and sends people to the
    /// program's page rather than to a library's.
    #[test]
    fn the_entry_is_the_installed_program_s() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let write = |path: &str, text: &str| {
            let path = root.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        };
        write("lyrn.toml", "[project]\nname = \"relay\"\nform = \"workspace\"\n");
        write(
            "Cargo.toml",
            "[workspace]\nmembers = [\"crates/*\"]\n\n[workspace.package]\nversion = \"0.10.0\"\nhomepage = \"https://owner.github.io/relay/\"\n",
        );
        write(
            "crates/relay/Cargo.toml",
            "[package]\nname = \"relay\"\nversion.workspace = true\nhomepage = \"https://owner.github.io/relay/client/\"\n",
        );
        write("crates/relay/src/main.rs", "fn main() {}\n");
        write(
            "crates/seal/Cargo.toml",
            "[package]\nname = \"seal\"\nversion.workspace = true\nhomepage = \"https://owner.github.io/relay/concepts/sealing/\"\n",
        );
        write("crates/seal/src/lib.rs", "");
        write(
            "crates/relay-server/Cargo.toml",
            "[package]\nname = \"relay-server\"\nversion.workspace = true\nhomepage.workspace = true\npublish = false\n",
        );
        write("crates/relay-server/src/main.rs", "fn main() {}\n");
        write(
            "release.toml",
            "[post.install]\nbin = \"relay-server\"\n\n[page]\nassets = [\"relay-server-{tag}-x86_64-unknown-linux-gnu.tar.gz\"]\n",
        );
        let repo = Repo::on_disk(root);
        let product = Product::read(&repo);
        let entry = super::entry(&repo, &product, "owner/relay");
        assert_eq!(entry["name"], "relay-server", "{entry}");
        assert_eq!(entry["bin"], "relay-server");
        assert_eq!(entry["homepage"], "https://owner.github.io/relay/");
    }

    #[test]
    fn a_program_in_both_lists_passes() {
        let listed = Ok(Some(format!("[{}]", entry())));
        match judge(&product(), "owner/demo", &entry(), lists(listed.clone(), listed)) {
            Outcome::Pass(text) => assert_eq!(text, "demo is in owner/homebrew-tap and owner/scoop-bucket"),
            other => panic!("{other:?}"),
        }
    }

    /// Missing from one list is a failure that hands over the entry to add;
    /// a list that is not there at all says so.
    #[test]
    fn a_program_missing_from_a_list_fails_with_its_entry() {
        let other = Ok(Some(r#"[{"name": "turnout", "repo": "owner/turnout", "bin": "turnout"}]"#.to_string()));
        let Outcome::Found(findings) = judge(&product(), "owner/demo", &entry(), lists(other, Ok(None))) else {
            panic!()
        };
        assert_eq!(findings.len(), 2);
        assert!(
            findings[0].what.contains("demo is not in owner/homebrew-tap's packages.json"),
            "{}",
            findings[0].what
        );
        assert!(findings[0].fix.as_deref().unwrap().contains("\"repo\": \"owner/demo\""));
        assert!(findings[1].what.contains("owner/scoop-bucket has no packages.json"));
    }

    /// The name and the program are what the README's lines install by, so
    /// a list that disagrees fails; a homepage elsewhere is looked at.
    #[test]
    fn a_list_that_disagrees_is_named() {
        let mut wrong = entry();
        wrong["bin"] = json!("dm");
        wrong["homepage"] = json!("https://elsewhere.dev");
        let listed = Ok(Some(format!("[{wrong}]")));
        let Outcome::Found(findings) = judge(&product(), "owner/demo", &entry(), lists(listed, Ok(Some(format!("[{}]", entry()))))) else {
            panic!()
        };
        let levels: Vec<_> = findings.iter().map(|f| f.level).collect();
        assert_eq!(levels, [crate::commands::doctor::Level::Fail, crate::commands::doctor::Level::Warn]);
        assert!(findings[0].what.contains("`bin`"));
    }
}
