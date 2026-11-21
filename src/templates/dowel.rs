//! Copies of dowel's registry components, for the forms that draw a UI.
//!
//! A product of the line presses dowel's Button rather than a `<button>` -
//! dowel-ui's lint refuses the raw element outside `components/ui/` - and its
//! sign-in screen is dowel's LoginScreen block. Registry components are
//! copied into a product and never imported from the package, so a generated
//! project has to arrive with the copies, and `lyrn new` works offline, so
//! they travel inside the binary.
//!
//! The files under `dowel/ui/` and `dowel/blocks/` are written by
//! `tools/vendor-dowel.mjs` from the registry catalogue of the dowel-ui
//! version in `dowel/VERSION`, byte for byte, with `dowel/catalogue.tsv`
//! saying what each item is made of. Two gates keep them honest: the tests
//! below refuse a template that pins a different dowel-ui, and the generated
//! project's own `tools/check-registry.mjs` compares every copy with the
//! package it installed - which lyrn's CI runs for each form that carries them.
//!
//! The same registry, read from a project's `node_modules`, is what `lyrn add`
//! takes copies from: one way from a name to the files it brings, whichever
//! dowel-ui the registry is of.

use std::path::{Path, PathBuf};

use crate::generate::{Contents, SourceFile};
use crate::model::Addon;

include!(concat!(env!("OUT_DIR"), "/marks.rs"));

/// The line's umbrella mark, which a project wears until it has one of its own.
pub const UMBRELLA: &str = "lacodda";

/// A product's masters, L, M and S, if the line has that product; the umbrella
/// mark is no product's.
pub fn product_mark(name: &str) -> Option<[&'static str; 3]> {
    MASTERS
        .iter()
        .find(|(product, ..)| *product == name && *product != UMBRELLA)
        .map(|(_, l, m, s)| [*l, *m, *s])
}

/// Whether a master is the umbrella mark's L, whatever its line endings.
pub fn is_umbrella_l(master: &[u8]) -> bool {
    let strip = |b: &[u8]| -> Vec<u8> { b.iter().copied().filter(|&c| c != b'\r').collect() };
    MASTERS
        .iter()
        .find(|(product, ..)| *product == UMBRELLA)
        .is_some_and(|(_, l, ..)| strip(master) == strip(l.as_bytes()))
}

/// The dowel-ui release the copies were taken from, as `tools/vendor-dowel.mjs`
/// wrote it.
#[cfg(test)]
pub fn version() -> &'static str {
    include_str!("dowel/VERSION").trim()
}

include!(concat!(env!("OUT_DIR"), "/catalogue.rs"));

/// One registry item as a template carries it.
#[derive(Debug, Clone, Copy)]
pub struct Primitive {
    /// Where it lands in the spa form: `components.json` maps dowel's `@ui/`
    /// target to `src/components/ui`, and `@components/` to `src/components`.
    pub path: &'static str,
    contents: &'static str,
}

impl Primitive {
    /// The file a form writes; `addon` when only that add-on needs it.
    pub fn source(self, addon: Option<Addon>) -> SourceFile {
        SourceFile {
            path: self.path,
            contents: Contents::Text(self.contents),
            executable: false,
            addon,
        }
    }
}

/// The copy of a registry item the binary carries, by its name in the
/// registry. A name the catalogue does not have is a template asking for a
/// copy `tools/vendor-dowel.mjs` was never told to take - a mistake in lyrn,
/// which every test of the form that asks finds.
pub fn copy(name: &str) -> Primitive {
    let (_, _, target, _, _, contents) = CATALOGUE
        .iter()
        .find(|(item, ..)| *item == name)
        .unwrap_or_else(|| panic!("lyrn carries no copy of dowel's `{name}` - add it to PRIMITIVES in tools/vendor-dowel.mjs"));
    let path = SPA_PLACES
        .iter()
        .find(|(t, _)| *t == *target)
        .map(|(_, path)| *path)
        .unwrap_or_else(|| panic!("`{target}` has no place in the spa form listed in SPA_PLACES"));
    Primitive { path, contents }
}

/// Where the spa form puts the copies its files are written with. Listed,
/// because a `SourceFile` path is `'static`; `copy` refuses a target missing
/// here, and a test holds the list to the catalogue's own placement.
const SPA_PLACES: &[(&str, &str)] = &[
    ("@ui/button.tsx", "src/components/ui/button.tsx"),
    ("@ui/field.tsx", "src/components/ui/field.tsx"),
    ("@ui/input.tsx", "src/components/ui/input.tsx"),
    ("@ui/panel.tsx", "src/components/ui/panel.tsx"),
    ("@ui/alert.tsx", "src/components/ui/alert.tsx"),
    ("@ui/password-field.tsx", "src/components/ui/password-field.tsx"),
    ("@components/blocks/login-screen.tsx", "src/components/blocks/login-screen.tsx"),
];

/// What an item of dowel's registry is, as far as writing it goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    /// A primitive: a copy under `components/ui`, held to the registry by
    /// the project's `check-registry`.
    Ui,
    /// A screen made of primitives: written once under `components/blocks`,
    /// and the product's to change from then on.
    Block,
    /// A set of other items and no file of its own: `app`, `forms`.
    Bundle,
    /// Anything else the registry has - the theme, the prose styles, a file
    /// for the assistant - which comes with the package rather than as a copy.
    Other(String),
}

/// One item of dowel's registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub name: String,
    pub kind: Kind,
    /// Where the registry puts its file - `@ui/button.tsx`,
    /// `@components/blocks/login-screen.tsx` - and the file's text.
    pub file: Option<(String, String)>,
    /// The items of the same registry it is made of.
    pub needs: Vec<String>,
    /// The npm packages it imports, as `name` or `name@range`.
    pub packages: Vec<String>,
}

/// A dowel-ui registry: the one inside the binary, or the one a project
/// installed.
#[derive(Debug, Clone)]
pub struct Catalogue {
    /// The dowel-ui release it is the registry of.
    pub version: String,
    items: Vec<Item>,
}

impl Catalogue {
    /// The items the binary carries, from the dowel-ui in `dowel/VERSION`:
    /// what `lyrn new` writes, offline.
    pub fn builtin() -> Catalogue {
        let items = CATALOGUE
            .iter()
            .map(|(name, kind, target, needs, packages, text)| Item {
                name: name.to_string(),
                kind: if *kind == "block" { Kind::Block } else { Kind::Ui },
                file: Some((target.to_string(), text.to_string())),
                needs: needs.iter().map(|s| s.to_string()).collect(),
                packages: packages.iter().map(|s| s.to_string()).collect(),
            })
            .collect();
        Catalogue {
            version: include_str!("dowel/VERSION").trim().to_string(),
            items,
        }
    }

    /// The registry of the dowel-ui installed under `frontend`: the copies a
    /// project's `check-registry` holds it to.
    pub fn installed(frontend: &Path) -> Result<Catalogue, String> {
        let package = frontend.join("node_modules").join("dowel-ui");
        let registry = std::fs::read_to_string(package.join("dist").join("registry.json")).map_err(|_| {
            format!(
                "dowel-ui is not installed in {} - `pnpm install` first: the copies come from the dowel-ui the project pins, the one `pnpm registry` holds them to",
                shown(frontend)
            )
        })?;
        let manifest = std::fs::read_to_string(package.join("package.json")).map_err(|e| format!("dowel-ui's package.json: {e}"))?;
        let version = serde_json::from_str::<serde_json::Value>(&manifest)
            .ok()
            .and_then(|m| m.get("version")?.as_str().map(str::to_string))
            .ok_or("dowel-ui's package.json names no version")?;
        Catalogue::parse(&registry, &version)
    }

    /// A registry as shadcn's format writes it: `items`, each with a `type`,
    /// `files`, `registryDependencies` as addresses of the same registry,
    /// and `dependencies` as npm specs.
    pub fn parse(json: &str, version: &str) -> Result<Catalogue, String> {
        let registry: serde_json::Value = serde_json::from_str(json).map_err(|e| format!("dowel-ui's registry is not JSON: {e}"))?;
        let entries = registry.get("items").and_then(|i| i.as_array()).ok_or("dowel-ui's registry has no items")?;
        let strings = |entry: &serde_json::Value, key: &str| -> Vec<String> {
            entry
                .get(key)
                .and_then(|v| v.as_array())
                .into_iter()
                .flatten()
                .filter_map(|v| v.as_str())
                .map(str::to_string)
                .collect()
        };
        let mut items = Vec::new();
        for entry in entries {
            let Some(name) = entry.get("name").and_then(|n| n.as_str()) else { continue };
            let kind = entry.get("type").and_then(|t| t.as_str()).unwrap_or_default();
            let files: Vec<(String, String)> = entry
                .get("files")
                .and_then(|f| f.as_array())
                .into_iter()
                .flatten()
                .filter_map(|file| {
                    let target = file.get("target").or_else(|| file.get("path"))?.as_str()?;
                    Some((target.to_string(), file.get("content")?.as_str()?.to_string()))
                })
                .collect();
            let kind = match (kind, files.len()) {
                ("registry:ui", 1) => Kind::Ui,
                ("registry:block", 1) => Kind::Block,
                ("registry:style", 0) => Kind::Bundle,
                (other, _) => Kind::Other(other.trim_start_matches("registry:").to_string()),
            };
            items.push(Item {
                name: name.to_string(),
                file: if matches!(kind, Kind::Ui | Kind::Block) {
                    files.into_iter().next()
                } else {
                    None
                },
                kind,
                needs: strings(entry, "registryDependencies").iter().map(|d| dependency_name(d)).collect(),
                packages: strings(entry, "dependencies"),
            });
        }
        Ok(Catalogue {
            version: version.to_string(),
            items,
        })
    }

    pub fn get(&self, name: &str) -> Option<&Item> {
        self.items.iter().find(|i| i.name == name)
    }

    /// Every block, by its registry name.
    pub fn blocks(&self) -> impl Iterator<Item = &Item> {
        self.items.iter().filter(|i| i.kind == Kind::Block)
    }

    /// `names` and every item they are made of, the parts before what is
    /// made of them, each once.
    pub fn closure(&self, names: &[String]) -> Result<Vec<&Item>, String> {
        let mut taken: Vec<&Item> = Vec::new();
        for name in names {
            self.take(name, &mut Vec::new(), &mut taken)?;
        }
        Ok(taken)
    }

    fn take<'a>(&'a self, name: &str, path: &mut Vec<String>, taken: &mut Vec<&'a Item>) -> Result<(), String> {
        if taken.iter().any(|i| i.name == name) {
            return Ok(());
        }
        if path.iter().any(|p| p == name) {
            return Err(format!(
                "dowel-ui {}'s registry goes round in a circle: {} -> {name}",
                self.version,
                path.join(" -> ")
            ));
        }
        let item = self.get(name).ok_or_else(|| match path.last() {
            Some(by) => format!("dowel-ui {} has no `{name}`, which `{by}` is made of", self.version),
            None => format!("dowel-ui {} has no `{name}`", self.version),
        })?;
        path.push(name.to_string());
        for part in &item.needs {
            self.take(part, path, taken)?;
        }
        path.pop();
        taken.push(item);
        Ok(())
    }
}

/// `https://lacodda.github.io/dowel/r/<name>.json` is how an item names
/// another of the same registry; a bare name is one already.
fn dependency_name(address: &str) -> String {
    address
        .rsplit('/')
        .next()
        .and_then(|last| last.strip_suffix(".json"))
        .unwrap_or(address)
        .to_string()
}

/// Where a project's `components.json` says the registry's places are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Places {
    /// `@ui/`: `src/components/ui`.
    pub ui: PathBuf,
    /// `@components/`: `src/components`.
    pub components: PathBuf,
}

impl Places {
    /// Read from a `components.json`, whose aliases are import paths: `@/`
    /// is `src/`, as every frontend of the line's stack sets it in
    /// `tsconfig.json`.
    pub fn read(components_json: &str) -> Result<Places, String> {
        let config: serde_json::Value = serde_json::from_str(components_json).map_err(|e| format!("components.json is not JSON: {e}"))?;
        let alias = |key: &str| -> Result<PathBuf, String> {
            let value = config
                .get("aliases")
                .and_then(|a| a.get(key))
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("components.json names no `aliases.{key}`"))?;
            let rest = value
                .strip_prefix("@/")
                .ok_or_else(|| format!("components.json's `aliases.{key}` is `{value}`, and lyrn knows where `@/` is - `src/` - and nothing else"))?;
            Ok(Path::new("src").join(rest))
        };
        Ok(Places {
            ui: alias("ui")?,
            components: alias("components")?,
        })
    }

    /// Where an item's file lands, relative to the frontend's root.
    pub fn of(&self, item: &Item) -> Result<Option<PathBuf>, String> {
        let Some((target, _)) = &item.file else { return Ok(None) };
        if let Some(rest) = target.strip_prefix("@ui/") {
            return Ok(Some(self.ui.join(rest)));
        }
        if let Some(rest) = target.strip_prefix("@components/") {
            return Ok(Some(self.components.join(rest)));
        }
        Err(format!("`{}` goes to `{target}`, a place a project does not have", item.name))
    }
}

/// A directory as a report names it: `.` rather than nothing.
fn shown(dir: &Path) -> String {
    if dir.as_os_str().is_empty() {
        ".".to_string()
    } else {
        dir.display().to_string()
    }
}

/// Every copy of a primitive the binary carries.
#[cfg(test)]
pub fn all() -> Vec<Primitive> {
    SPA_PLACES
        .iter()
        .map(|(target, _)| {
            let name = target.rsplit('/').next().unwrap().trim_end_matches(".tsx");
            copy(name)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Form, TemplateManifest};
    use crate::templates::{manifest_for, sources_for};

    /// A template that moves its dowel-ui pin without the copies being taken
    /// again would install one version and carry another's components. The
    /// generated project's registry gate would say so - but only in lyrn's CI,
    /// after the push. This says so before the commit.
    #[test]
    fn every_template_pins_the_dowel_ui_the_copies_came_from() {
        let wanted = format!("\"dowel-ui\": \"^{}\"", version());
        for form in Form::ALL {
            for source in sources_for(*form) {
                let Contents::Text(text) = source.contents else { continue };
                if !source.path.ends_with("package.json") || !text.contains("\"dowel-ui\"") {
                    continue;
                }
                assert!(
                    text.contains(&wanted),
                    "{form}: {} pins a dowel-ui other than the vendored {} - run `node tools/vendor-dowel.mjs <version>`",
                    source.path,
                    version()
                );
            }
        }
    }

    /// A copy is dowel's text, not a template: a `{{` in some future component
    /// would otherwise be read as a placeholder, and the generation would fail
    /// or - worse - quietly rewrite the component.
    #[test]
    fn every_copy_a_form_writes_travels_verbatim() {
        for form in Form::ALL {
            let manifest: TemplateManifest = toml::from_str(manifest_for(*form)).unwrap();
            for source in sources_for(*form) {
                // `ends_with`: a service carries the spa's copies under `frontend/`.
                if all().iter().any(|p| source.path.ends_with(p.path)) {
                    assert!(
                        manifest.verbatim.iter().any(|v| v == source.path),
                        "{form}: {} is a copy of a dowel component and must be listed in `verbatim`",
                        source.path
                    );
                }
            }
        }
    }

    #[test]
    fn the_vendored_version_is_a_version() {
        let parts: Vec<&str> = version().split('.').collect();
        assert_eq!(parts.len(), 3, "`{}` is not major.minor.patch", version());
        assert!(parts.iter().all(|p| p.parse::<u32>().is_ok()), "`{}` is not numeric", version());
    }

    #[test]
    fn every_copy_is_a_component() {
        for (name, .., text) in CATALOGUE {
            assert!(text.contains("export "), "{name} exports nothing");
        }
    }

    /// The places the spa form writes its copies at are the ones its own
    /// `components.json` gives them: the list is a convenience for `'static`
    /// paths, not a second opinion.
    #[test]
    fn the_spa_form_puts_each_copy_where_its_components_json_says() {
        let places = Places::read(include_str!("spa/components.json.tmpl")).unwrap();
        let catalogue = Catalogue::builtin();
        for (target, path) in SPA_PLACES {
            let item = catalogue
                .items
                .iter()
                .find(|i| i.file.as_ref().is_some_and(|(t, _)| t == target))
                .unwrap_or_else(|| panic!("the catalogue has nothing at {target}"));
            assert_eq!(places.of(item).unwrap(), Some(PathBuf::from(path)), "{}", item.name);
        }
    }

    /// Every block the binary carries can be written whole: each part it is
    /// made of is carried too, so `--with block:<x>` never needs the network.
    #[test]
    fn every_block_is_carried_with_its_parts() {
        let catalogue = Catalogue::builtin();
        let blocks: Vec<String> = catalogue.blocks().map(|b| b.name.clone()).collect();
        assert!(blocks.contains(&"login-screen".to_string()), "{blocks:?}");
        for block in &blocks {
            let closure = catalogue.closure(std::slice::from_ref(block)).unwrap();
            assert_eq!(closure.last().unwrap().name, *block, "the block comes after its parts");
        }
    }

    fn item(name: &str, needs: &[&str]) -> String {
        let needs: Vec<String> = needs.iter().map(|n| format!("\"https://example.org/r/{n}.json\"")).collect();
        format!(
            r#"{{"name": "{name}", "type": "registry:ui", "registryDependencies": [{}], "dependencies": ["dowel-ui@^1.0.0"],
                "files": [{{"path": "registry/ui/{name}.tsx", "target": "@ui/{name}.tsx", "content": "export const {name} = 1"}}]}}"#,
            needs.join(", ")
        )
    }

    /// Parts before what is made of them, each once - however many items
    /// share a part.
    #[test]
    fn a_closure_lists_the_parts_first_and_each_once() {
        let json = format!(
            r#"{{"items": [{}, {}, {}, {}, {{"name": "theme", "type": "registry:style", "files": [{{"path": "theme.css", "target": "~/dowel/theme.css", "content": ""}}]}},
                {{"name": "app", "type": "registry:style", "registryDependencies": ["https://example.org/r/frame.json"]}}]}}"#,
            item("button", &[]),
            item("scroll", &[]),
            item("frame", &["button", "scroll"]),
            item("dialog", &["button"]),
        );
        let catalogue = Catalogue::parse(&json, "1.0.0").unwrap();
        let names = |list: &[&str]| -> Vec<String> {
            let wanted: Vec<String> = list.iter().map(|s| s.to_string()).collect();
            catalogue.closure(&wanted).unwrap().iter().map(|i| i.name.clone()).collect()
        };
        assert_eq!(names(&["frame", "dialog"]), ["button", "scroll", "frame", "dialog"]);
        assert_eq!(names(&["app"]), ["button", "scroll", "frame", "app"]);
        assert_eq!(catalogue.get("app").unwrap().kind, Kind::Bundle);
        assert_eq!(catalogue.get("theme").unwrap().kind, Kind::Other("style".into()));
        assert_eq!(catalogue.get("frame").unwrap().packages, ["dowel-ui@^1.0.0"]);
        let missing = catalogue.closure(&["nothing".to_string()]).unwrap_err();
        assert!(missing.contains("dowel-ui 1.0.0 has no `nothing`"), "{missing}");
    }

    #[test]
    fn a_part_the_registry_does_not_have_is_named_with_what_needed_it() {
        let json = format!(r#"{{"items": [{}]}}"#, item("frame", &["gone"]));
        let catalogue = Catalogue::parse(&json, "1.0.0").unwrap();
        let error = catalogue.closure(&["frame".to_string()]).unwrap_err();
        assert!(error.contains("no `gone`, which `frame` is made of"), "{error}");
    }

    #[test]
    fn a_place_comes_from_the_aliases_and_nothing_else() {
        let places = Places::read(r#"{"aliases": {"ui": "@/parts/ui", "components": "@/parts"}}"#).unwrap();
        let mut block = Item {
            name: "login-screen".into(),
            kind: Kind::Block,
            file: Some(("@components/blocks/login-screen.tsx".into(), String::new())),
            needs: vec![],
            packages: vec![],
        };
        assert_eq!(places.of(&block).unwrap(), Some(PathBuf::from("src/parts/blocks/login-screen.tsx")));
        block.file = Some(("~/dowel/theme.css".into(), String::new()));
        assert!(places.of(&block).is_err());
        let elsewhere = Places::read(r#"{"aliases": {"ui": "~/ui", "components": "@/components"}}"#).unwrap_err();
        assert!(elsewhere.contains("`~/ui`"), "{elsewhere}");
    }
}
