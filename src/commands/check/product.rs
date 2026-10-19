//! The product a repository ships, read from its manifests: what it is
//! called, which manifests carry its version, what it says it is.
//!
//! Found by the standard's layout rather than listed by hand. A list is the
//! thing that goes stale: the manifest nobody wrote into it is the one left
//! at the old number. So the rule is stated once - a manifest belongs to the
//! product unless it says otherwise - and the ways a manifest says otherwise
//! are few: it is a test's material or an example, it is private and keeps a
//! placeholder number, or it is published under a name of its own and
//! released on its own.

use std::path::Path;

use serde_json::Value;

use crate::commands::doctor::{Repo, is_test_material};
use crate::repo;

/// Which file format a manifest is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Cargo,
    Package,
    Tauri,
}

/// One manifest of the repository, as far as the shopfronts care.
#[derive(Debug, Clone)]
pub struct Manifest {
    /// `/`-separated, from the top of the repository.
    pub path: String,
    pub kind: Kind,
    pub name: Option<String>,
    /// The version written in this file; `None` when it has none of its own.
    pub version: Option<String>,
    /// `version.workspace = true`: the number lives in the workspace.
    pub inherits: bool,
    /// `[workspace.package] version`, for a workspace's top manifest.
    pub workspace_version: Option<String>,
    /// Whether a registry would take it: not `private`, not `publish = false`.
    pub publishable: bool,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub documentation: Option<String>,
    /// Cargo's `readme`, when it names a file.
    pub readme: Option<String>,
    /// The programs a Cargo package builds.
    pub bins: Vec<String>,
    pub uses_clap: bool,
    /// The release an npm wrapper downloads its binary from, by where it
    /// says so: `lyrn.binary = v2.12.0`.
    pub binary_pins: Vec<(String, String)>,
}

impl Manifest {
    pub fn dir(&self) -> &str {
        self.path.rsplit_once('/').map_or(".", |(dir, _)| dir)
    }

    pub fn is_root(&self) -> bool {
        !self.path.contains('/')
    }
}

/// The product, and every manifest that speaks for it.
#[derive(Debug, Clone)]
pub struct Product {
    pub name: String,
    pub manifests: Vec<Manifest>,
    /// `owner/name` on GitHub, from the remote or a manifest's `repository`.
    pub github: Option<String>,
}

/// A number a private package keeps to say it has none of its own.
pub fn is_placeholder(version: &str) -> bool {
    matches!(version, "0.0.0" | "0.0.1")
}

impl Product {
    pub fn read(repo: &Repo) -> Product {
        let manifests: Vec<Manifest> = repo
            .files
            .iter()
            .filter(|f| !is_test_material(f) && !in_examples(f))
            .filter_map(|f| read_manifest(&repo.root, f))
            .collect();
        let facts = repo.facts.as_ref().ok();
        // The remote first; else whatever manifest names the repository, the
        // top ones before the rest.
        let github = repo::origin_repo(&repo.root).or_else(|| {
            let (top, rest): (Vec<&Manifest>, Vec<&Manifest>) = manifests.iter().partition(|m| m.is_root());
            top.into_iter().chain(rest).find_map(|m| manifest_repository(&repo.root, m))
        });
        // A private workspace's package.json is named for the workspace, not
        // the product: lyrn.toml and a top crate say the product's name, and
        // after them the repository does.
        let cargo_name = facts
            .and_then(|f| f.cargo.as_ref())
            .and_then(|c| c.get("package")?.get("name")?.as_str().map(str::to_string));
        let declared = facts.filter(|f| f.has_lyrn_toml).and_then(|f| f.name.clone());
        let name = declared
            .or(cargo_name)
            .or_else(|| github.as_ref().and_then(|g| g.split('/').nth(1).map(str::to_string)))
            .or_else(|| facts.and_then(|f| f.name.clone()))
            .unwrap_or_else(|| {
                repo.root
                    .file_name()
                    .map_or_else(|| "project".to_string(), |n| n.to_string_lossy().into_owned())
            });
        Product { name, manifests, github }
    }

    /// Whether a package name is this product's: its own name, a name built
    /// on it (`kasl-cli`, `dowel-ui`), or either under an npm scope.
    pub fn owns(&self, name: &str) -> bool {
        let unscoped = name.rsplit('/').next().unwrap_or(name);
        unscoped == self.name || unscoped.starts_with(&format!("{}-", self.name))
    }

    /// The manifests whose version is the product's, each with the number it
    /// writes.
    pub fn versioned(&self) -> Vec<(&Manifest, &str)> {
        self.manifests
            .iter()
            .filter_map(|m| {
                let version = m.version.as_deref().or(m.workspace_version.as_deref())?;
                if is_placeholder(version) && !m.publishable {
                    return None;
                }
                let own_product = m.publishable && m.name.as_deref().is_some_and(|n| !self.owns(n)) && !m.is_root();
                (!own_product).then_some((m, version))
            })
            .collect()
    }

    /// The version the product is at, and the manifest that says it first:
    /// the top Cargo manifest, else the top package.json, else the first
    /// package named after the product.
    pub fn version(&self) -> Option<(&Manifest, &str)> {
        let versioned = self.versioned();
        let rank = |m: &Manifest| match (m.is_root() && m.kind != Kind::Tauri, m.kind, m.name.as_deref() == Some(self.name.as_str())) {
            (true, Kind::Cargo, _) => 0,
            (true, Kind::Package, _) => 1,
            (_, _, true) => 2,
            (_, Kind::Cargo, _) => 3,
            _ => 4,
        };
        versioned.into_iter().min_by_key(|(m, _)| rank(m))
    }

    /// What the product says it is, and where it says it: the top Cargo
    /// manifest's package, else the top package.json of an application. A
    /// workspace has neither - each crate describes itself, and none speaks
    /// for the whole.
    pub fn description(&self) -> Option<(&Manifest, &str)> {
        let described = |m: &&Manifest| m.is_root() && m.description.as_deref().is_some_and(|d| !d.trim().is_empty());
        self.manifests
            .iter()
            .filter(described)
            .find(|m| m.kind == Kind::Cargo)
            .or_else(|| self.manifests.iter().filter(described).find(|m| m.kind == Kind::Package))
            .map(|m| (m, m.description.as_deref().unwrap_or_default()))
    }

    /// The crate an npm wrapper installs: named as the wrapper is, or as it
    /// is without its `-cli`.
    pub fn wrapped_crate(&self, wrapper: &Manifest) -> Option<&Manifest> {
        let name = wrapper.name.as_deref()?;
        let unscoped = name.rsplit('/').next().unwrap_or(name);
        let bare = unscoped.strip_suffix("-cli").unwrap_or(unscoped);
        self.manifests
            .iter()
            .filter(|m| m.kind == Kind::Cargo && m.publishable)
            .find(|m| m.name.as_deref() == Some(unscoped) || m.name.as_deref() == Some(bare))
    }

    /// Whether a manifest shows the product itself rather than a part of it:
    /// the product's own package, its `-cli` crate or npm wrapper, the top
    /// package.json of an application. A library the product publishes
    /// beside itself describes that library.
    pub fn presents(&self, manifest: &Manifest) -> bool {
        let unscoped = manifest.name.as_deref().map(|n| n.rsplit('/').next().unwrap_or(n));
        let named = unscoped.is_some_and(|n| n == self.name || n == format!("{}-cli", self.name));
        (named && (manifest.publishable || manifest.is_root()))
            || (manifest.kind == Kind::Package && manifest.is_root())
            || (manifest.path.starts_with("npm/") && manifest.publishable)
    }

    /// The command-line programs the product builds: a Cargo package of its
    /// own that parses its arguments with clap.
    pub fn programs(&self) -> Vec<(&Manifest, &str)> {
        self.manifests
            .iter()
            .filter(|m| m.kind == Kind::Cargo && m.uses_clap)
            .filter(|m| m.is_root() || m.name.as_deref().is_some_and(|n| self.owns(n)))
            .flat_map(|m| m.bins.iter().map(move |b| (m, b.as_str())))
            .collect()
    }
}

fn in_examples(path: &str) -> bool {
    path.split('/').any(|part| part == "examples" || part == "example")
}

fn read_manifest(root: &Path, path: &str) -> Option<Manifest> {
    let name = path.rsplit('/').next()?;
    let text = std::fs::read_to_string(root.join(path)).ok()?;
    match name {
        "Cargo.toml" => read_cargo(root, path, &text),
        "package.json" => read_package(path, &text),
        "tauri.conf.json" => read_tauri(path, &text),
        _ => None,
    }
}

fn read_cargo(root: &Path, path: &str, text: &str) -> Option<Manifest> {
    let table: toml::Table = toml::from_str(text).ok()?;
    let package = table.get("package").and_then(|p| p.as_table());
    let string = |key: &str| package.and_then(|p| p.get(key)).and_then(|v| v.as_str()).map(str::to_string);
    let inherited = |key: &str| {
        package
            .and_then(|p| p.get(key))
            .and_then(|v| v.as_table())
            .is_some_and(|t| t.get("workspace").and_then(|w| w.as_bool()) == Some(true))
    };
    let workspace_version = table
        .get("workspace")
        .and_then(|w| w.get("package"))
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let workspace_string = |key: &str| {
        table
            .get("workspace")
            .and_then(|w| w.get("package"))
            .and_then(|p| p.get(key))
            .and_then(|v| v.as_str())
            .map(str::to_string)
    };
    let publishable = package.is_some_and(|p| !matches!(p.get("publish"), Some(toml::Value::Boolean(false))));
    let dir = path.rsplit_once('/').map_or("", |(dir, _)| dir);
    let package_name = string("name");
    let mut bins: Vec<String> = table
        .get("bin")
        .and_then(|b| b.as_array())
        .into_iter()
        .flatten()
        .filter_map(|b| b.get("name").and_then(|n| n.as_str()).map(str::to_string))
        .collect();
    if bins.is_empty() && package.is_some() && root.join(dir).join("src").join("main.rs").is_file() {
        bins.extend(package_name.clone());
    }
    let depends = |krate: &str| {
        ["dependencies", "build-dependencies"]
            .iter()
            .any(|key| table.get(*key).and_then(|d| d.as_table()).is_some_and(|d| d.contains_key(krate)))
    };
    Some(Manifest {
        path: path.to_string(),
        kind: Kind::Cargo,
        name: package_name,
        version: string("version"),
        inherits: inherited("version"),
        workspace_version,
        publishable: publishable && package.is_some(),
        description: string("description").or_else(|| inherited("description").then(|| workspace_string("description")).flatten()),
        homepage: string("homepage").or_else(|| inherited("homepage").then(|| workspace_string("homepage")).flatten()),
        documentation: string("documentation"),
        readme: string("readme"),
        bins,
        uses_clap: depends("clap"),
        binary_pins: Vec::new(),
    })
}

fn read_package(path: &str, text: &str) -> Option<Manifest> {
    let json: Value = serde_json::from_str(text).ok()?;
    let string = |key: &str| json.get(key).and_then(Value::as_str).map(str::to_string);
    let private = json.get("private").and_then(Value::as_bool) == Some(true);
    // A wrapper pins the release it downloads under a key of its own:
    // `"lyrn": { "binary": "v2.12.0" }`.
    let binary_pins = json
        .as_object()
        .into_iter()
        .flatten()
        .filter_map(|(key, value)| Some((format!("{key}.binary"), value.get("binary")?.as_str()?.to_string())))
        .collect();
    Some(Manifest {
        path: path.to_string(),
        kind: Kind::Package,
        name: string("name"),
        version: string("version"),
        inherits: false,
        workspace_version: None,
        publishable: !private,
        description: string("description"),
        homepage: string("homepage"),
        documentation: None,
        readme: None,
        bins: Vec::new(),
        uses_clap: false,
        binary_pins,
    })
}

fn read_tauri(path: &str, text: &str) -> Option<Manifest> {
    let json: Value = serde_json::from_str(text).ok()?;
    // Tauri also takes a path to a package.json here, and then the number is
    // that file's.
    let version = json
        .get("version")
        .and_then(Value::as_str)
        .filter(|v| !v.ends_with(".json"))
        .map(str::to_string);
    Some(Manifest {
        path: path.to_string(),
        kind: Kind::Tauri,
        name: json.get("productName").and_then(Value::as_str).map(str::to_string),
        version,
        inherits: false,
        workspace_version: None,
        publishable: false,
        description: None,
        homepage: None,
        documentation: None,
        readme: None,
        bins: Vec::new(),
        uses_clap: false,
        binary_pins: Vec::new(),
    })
}

fn manifest_repository(root: &Path, manifest: &Manifest) -> Option<String> {
    let text = std::fs::read_to_string(root.join(&manifest.path)).ok()?;
    let url = match manifest.kind {
        Kind::Cargo => {
            let table: toml::Table = toml::from_str(&text).ok()?;
            table
                .get("package")
                .and_then(|p| p.get("repository"))
                .or_else(|| table.get("workspace").and_then(|w| w.get("package")).and_then(|p| p.get("repository")))
                .and_then(|v| v.as_str())
                .map(str::to_string)?
        }
        _ => {
            let json: Value = serde_json::from_str(&text).ok()?;
            match json.get("repository")? {
                Value::String(url) => url.clone(),
                other => other.get("url")?.as_str()?.to_string(),
            }
        }
    };
    repo::github_repo(url.trim_start_matches("git+"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(path: &str, name: &str, version: Option<&str>, publishable: bool) -> Manifest {
        Manifest {
            path: path.into(),
            kind: if path.ends_with("Cargo.toml") {
                Kind::Cargo
            } else if path.ends_with("tauri.conf.json") {
                Kind::Tauri
            } else {
                Kind::Package
            },
            name: Some(name.into()),
            version: version.map(str::to_string),
            inherits: false,
            workspace_version: None,
            publishable,
            description: None,
            homepage: None,
            documentation: None,
            readme: None,
            bins: Vec::new(),
            uses_clap: false,
            binary_pins: Vec::new(),
        }
    }

    fn product(manifests: Vec<Manifest>) -> Product {
        Product {
            name: "demo".into(),
            manifests,
            github: None,
        }
    }

    #[test]
    fn a_manifest_belongs_to_the_product_unless_it_says_otherwise() {
        let product = product(vec![
            manifest("Cargo.toml", "demo", Some("1.2.0"), true),
            manifest("npm/package.json", "demo-cli", Some("1.2.0"), true),
            manifest("web/package.json", "demo-web", Some("1.1.0"), false),
            // A placeholder in a private package: no version of its own.
            manifest("docs/package.json", "demo-docs", Some("0.0.1"), false),
            // Published under a name of its own: released on its own.
            manifest("crates/seal/Cargo.toml", "lacodda-seal", Some("0.1.0"), true),
            manifest("src-tauri/tauri.conf.json", "Demo", Some("1.2.0"), false),
        ]);
        let versioned: Vec<&str> = product.versioned().iter().map(|(m, _)| m.path.as_str()).collect();
        assert_eq!(versioned, ["Cargo.toml", "npm/package.json", "web/package.json", "src-tauri/tauri.conf.json"]);
        assert_eq!(product.version().map(|(m, v)| (m.path.as_str(), v)), Some(("Cargo.toml", "1.2.0")));
    }

    #[test]
    fn without_a_top_manifest_the_package_named_after_the_product_says_the_version() {
        let product = product(vec![
            manifest("package.json", "demo-workspace", None, false),
            manifest("stand/package.json", "demo-stand", Some("0.0.0"), false),
            manifest("packages/demo/package.json", "demo-ui", Some("0.37.0"), true),
        ]);
        assert_eq!(
            product.version().map(|(m, v)| (m.path.as_str(), v)),
            Some(("packages/demo/package.json", "0.37.0"))
        );
    }

    #[test]
    fn a_name_is_the_product_s_when_it_is_built_on_it() {
        let product = product(Vec::new());
        for name in ["demo", "demo-cli", "@lacodda/demo", "@lacodda/demo-editor"] {
            assert!(product.owns(name), "{name}");
        }
        for name in ["demolition", "lacodda-seal", "other-demo"] {
            assert!(!product.owns(name), "{name}");
        }
    }
}
