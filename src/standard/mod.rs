//! The line's standard, version by version: what each one changed, and the
//! template of every file of it that each form writes.
//!
//! A repository records the standard it was generated or adopted at in
//! `lyrn.toml`. Catching it up with a newer one is a three-way merge: what the
//! standard wrote then, what the repository has now, and what the standard
//! writes today. The middle one is the repository's. Today's is the templates
//! inside the binary; every earlier one is kept here, in an archive that grows
//! by one entry per version and never changes once a release has carried it.
//!
//! The archive keeps templates, not rendered files. A repository's name,
//! description and accent are its own, so the version it is on is rendered
//! with them at the moment of the upgrade - the same way as the new one, and
//! a line both render alike is a line neither changed.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use serde::Deserialize;

use crate::generate::{Body, TemplateFile};
use crate::model::{Form, TemplateManifest};

include!(concat!(env!("OUT_DIR"), "/standard_archive.rs"));

/// One version of the standard, as the archive keeps it.
#[derive(Debug, Clone)]
pub struct Revision {
    pub version: String,
    /// What it changed, one line each, in the words the upgrade commit and
    /// the documentation repeat.
    pub changes: Vec<String>,
    /// Files written as they are, with no placeholder substituted.
    verbatim: Vec<String>,
    /// Form, then the file's path, then the hash of its template.
    files: BTreeMap<String, BTreeMap<String, String>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Index {
    changes: Vec<String>,
    #[serde(default)]
    verbatim: Vec<String>,
    #[serde(default)]
    files: BTreeMap<String, BTreeMap<String, String>>,
}

impl Revision {
    fn parse(version: &str, text: &str) -> Self {
        let index: Index = toml::from_str(text).unwrap_or_else(|e| panic!("the archive of the standard {version} does not parse: {e}"));
        Self {
            version: version.to_string(),
            changes: index.changes,
            verbatim: index.verbatim,
            files: index.files,
        }
    }

    /// The templates of this version's files for `form`, as the generator
    /// reads them; `None` when the form wrote none of it.
    pub fn files(&self, form: Form) -> Option<Vec<TemplateFile>> {
        let files = self.files.get(form.as_str())?;
        Some(
            files
                .iter()
                .map(|(path, hash)| TemplateFile {
                    path: path.clone(),
                    body: Body::Text(blob(hash).to_string()),
                    executable: false,
                    addon: None,
                })
                .collect(),
        )
    }

    /// What the templates are rendered with: of a manifest, only what is
    /// written verbatim matters to a file of the standard.
    pub fn manifest(&self) -> TemplateManifest {
        TemplateManifest {
            standard: self.version.clone(),
            verbatim: self.verbatim.clone(),
            ..TemplateManifest::default()
        }
    }
}

fn blob(hash: &str) -> &'static str {
    BLOBS
        .iter()
        .find(|(h, _)| *h == hash)
        .map(|(_, text)| *text)
        .unwrap_or_else(|| panic!("the archive names a template {hash} it does not keep"))
}

/// Every version in the archive, oldest first.
pub fn revisions() -> Vec<Revision> {
    let mut all: Vec<Revision> = INDEXES.iter().map(|(version, text)| Revision::parse(version, text)).collect();
    all.sort_by(|a, b| order(&a.version, &b.version));
    all
}

/// One version, if the archive keeps it.
pub fn find(version: &str) -> Option<Revision> {
    revisions().into_iter().find(|r| r.version == version)
}

/// The version the built-in forms write.
pub fn current() -> String {
    crate::template::builtin_manifest(Form::Spa).standard
}

/// Versions in order: `2026.9` before `2026.10`, and `2026.11` before
/// `2026.11.1`, which a second change in one month would be.
pub fn order(a: &str, b: &str) -> Ordering {
    let parts = |v: &str| v.split('.').map(|p| p.parse::<u32>().unwrap_or(0)).collect::<Vec<_>>();
    parts(a).cmp(&parts(b))
}

/// The versions after `from` up to `to`, each with what it changed.
pub fn between(from: &str, to: &str) -> Vec<Revision> {
    revisions()
        .into_iter()
        .filter(|r| order(&r.version, from) == Ordering::Greater && order(&r.version, to) != Ordering::Greater)
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::templates;

    /// The versions a release has carried, and the hash of each one's index.
    ///
    /// A version leaves the workbench when the version after it is started: the
    /// archive test then refuses until the one before is listed here, and from
    /// then on refuses any change to it. A repository stamped with a version has
    /// that version's files; a version that changed afterwards would hand the
    /// merge an ancestor the repository never had.
    const FROZEN: &[(&str, &str)] = &[
        ("2026.10", "d8876be1441d5162"),
        ("2026.11", "989f06e993e280cc"),
        ("2026.11.1", "49d26bf7b43b015b"),
    ];

    /// The standard the built-in forms write, by form: path, then template.
    fn written_now() -> (BTreeMap<String, BTreeMap<String, String>>, Vec<String>) {
        let mut forms = BTreeMap::new();
        let mut verbatim: Vec<String> = Vec::new();
        for form in Form::ALL.iter().filter(|f| f.adoptable()) {
            let manifest = crate::template::builtin_manifest(*form);
            let files: Vec<TemplateFile> = templates::sources_for(*form).into_iter().map(TemplateFile::from).collect();
            let mut by_path = BTreeMap::new();
            for file in templates::standard_files(&files) {
                let Body::Text(text) = file.body else {
                    panic!("{form}: `{}` is a file of the standard and not text", file.path)
                };
                assert!(!file.executable, "{form}: `{}` is a file of the standard and executable", file.path);
                if manifest.verbatim.contains(&file.path) && !verbatim.contains(&file.path) {
                    verbatim.push(file.path.clone());
                }
                by_path.insert(file.path, text);
            }
            forms.insert(form.as_str().to_string(), by_path);
        }
        // A path is verbatim for every form or for none: the archive says it
        // once for the version.
        for form in Form::ALL.iter().filter(|f| f.adoptable()) {
            let manifest = crate::template::builtin_manifest(*form);
            for path in &verbatim {
                if forms[form.as_str()].contains_key(path) {
                    assert!(manifest.verbatim.contains(path), "{form} renders `{path}`, which another form writes verbatim");
                }
            }
        }
        verbatim.sort();
        (forms, verbatim)
    }

    fn hash(text: &str) -> String {
        format!("{:016x}", crate::hash::fnv(text.as_bytes()))
    }

    fn archive_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("standard").join("archive")
    }

    /// The index of a version, as the archive writes it.
    fn index_text(version: &str, changes: &[String], verbatim: &[String], forms: &BTreeMap<String, BTreeMap<String, String>>) -> String {
        let quote = |s: &str| toml::Value::String(s.to_string()).to_string();
        let mut out = format!(
            "# The line's standard {version}: what it changed, and the template of each\n\
             # of its files by form. `lyrn upgrade` renders the version a repository is\n\
             # on from here, as the common ancestor of a three-way merge.\n\
             #\n\
             # Written by `LYRN_BLESS=1 cargo test standard`, all but `changes`, which\n\
             # is said by hand. Once a release has carried a version it never changes:\n\
             # a later change to a file of the standard is a new version.\n\n\
             changes = [\n"
        );
        for change in changes {
            out.push_str(&format!("    {},\n", quote(change)));
        }
        out.push_str("]\n\n# Files written as they are, with no placeholder substituted.\nverbatim = [");
        out.push_str(&verbatim.iter().map(|v| quote(v)).collect::<Vec<_>>().join(", "));
        out.push_str("]\n");
        for (form, files) in forms {
            // A form's name is a bare key; a path, with its dots, is quoted.
            out.push_str(&format!("\n[files.{form}]\n"));
            for (path, text) in files {
                out.push_str(&format!("{} = {}\n", quote(path), quote(&hash(text))));
            }
        }
        out
    }

    /// Write a version into the archive: its index, and every template it
    /// keeps that the archive does not have yet. Templates no version names
    /// any more are taken out.
    fn write(version: &str, changes: &[String], verbatim: &[String], forms: &BTreeMap<String, BTreeMap<String, String>>) {
        let dir = archive_dir();
        let blobs = dir.join("blobs");
        std::fs::create_dir_all(&blobs).unwrap();
        for text in forms.values().flat_map(BTreeMap::values) {
            let path = blobs.join(format!("{}.tmpl", hash(text)));
            if !path.exists() {
                std::fs::write(&path, text).unwrap();
            }
        }
        std::fs::write(dir.join(format!("{version}.toml")), index_text(version, changes, verbatim, forms)).unwrap();

        let mut named = std::collections::BTreeSet::new();
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|e| e == "toml") {
                let index: Index = toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
                named.extend(index.files.into_values().flat_map(BTreeMap::into_values));
            }
        }
        for entry in std::fs::read_dir(&blobs).unwrap() {
            let path = entry.unwrap().path();
            let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
            if !named.contains(&stem) {
                std::fs::remove_file(path).unwrap();
            }
        }
    }

    /// The archive holds the version the templates write, file for file - so
    /// a template changed without the archive is a red test, not an upgrade
    /// that merges against an ancestor nobody wrote.
    ///
    /// After a deliberate change to a file of the standard:
    /// `LYRN_BLESS=1 cargo test standard`, then say what changed in the
    /// version's `changes`, then run the tests again.
    #[test]
    fn the_archive_keeps_the_standard_the_templates_write() {
        let version = current();
        let (forms, verbatim) = written_now();
        let frozen = FROZEN.iter().find(|(v, _)| *v == version);

        if std::env::var_os("LYRN_BLESS").is_some() {
            assert!(
                frozen.is_none(),
                "the standard {version} has been released and is frozen - give the templates a new version of the standard"
            );
            let path = archive_dir().join(format!("{version}.toml"));
            let changes = std::fs::read_to_string(&path)
                .ok()
                .map(|text| toml::from_str::<Index>(&text).unwrap().changes)
                .unwrap_or_default();
            write(&version, &changes, &verbatim, &forms);
            panic!("wrote the archive of the standard {version}; say what changed in `changes`, then run the tests again");
        }

        let revision = find(&version)
            .unwrap_or_else(|| panic!("the archive has no standard {version}, which the templates write - run `LYRN_BLESS=1 cargo test standard`"));
        let archived: BTreeMap<String, BTreeMap<String, String>> = revision.files.clone();
        let expected: BTreeMap<String, BTreeMap<String, String>> = forms
            .iter()
            .map(|(form, files)| (form.clone(), files.iter().map(|(p, t)| (p.clone(), hash(t))).collect()))
            .collect();
        assert!(
            archived == expected && revision.verbatim == verbatim,
            "the templates write a standard {version} that differs from the archive's - run `LYRN_BLESS=1 cargo test standard` and review the diff"
        );
        assert!(!revision.changes.is_empty(), "the archive does not say what the standard {version} changed");
    }

    #[test]
    fn a_released_version_never_changes() {
        let current = current();
        for (version, text) in INDEXES {
            let hash = hash(text);
            match FROZEN.iter().find(|(v, _)| v == version) {
                Some((_, frozen)) => assert_eq!(
                    *frozen, hash,
                    "the standard {version} has been released, and its archive changed - put the change in a new version"
                ),
                None => assert_eq!(
                    *version, current,
                    "the standard {version} is behind {current} and not frozen - add (\"{version}\", \"{hash}\") to FROZEN"
                ),
            }
        }
        for (version, _) in FROZEN {
            assert!(find(version).is_some(), "FROZEN names a standard {version} the archive does not keep");
        }
    }

    /// Every template the archive keeps is named by a version, and every one
    /// a version names is kept.
    #[test]
    fn the_archive_keeps_what_it_names_and_nothing_else() {
        let named: std::collections::BTreeSet<String> = revisions()
            .iter()
            .flat_map(|r| r.files.values().flat_map(|files| files.values().cloned()).collect::<Vec<_>>())
            .collect();
        let kept: std::collections::BTreeSet<String> = BLOBS.iter().map(|(h, _)| h.to_string()).collect();
        assert_eq!(named, kept);
        for (h, text) in BLOBS {
            assert_eq!(*h, hash(text), "the template kept as {h} has another hash - it was edited by hand");
            assert!(!text.contains('\r'), "the template {h} has CRLF line endings");
        }
    }

    #[test]
    fn every_form_writes_one_standard() {
        let current = current();
        for form in Form::ALL {
            assert_eq!(crate::template::builtin_manifest(*form).standard, current, "{form} writes another standard");
        }
    }

    #[test]
    fn versions_are_ordered_by_their_numbers() {
        assert_eq!(order("2026.9", "2026.10"), Ordering::Less);
        assert_eq!(order("2026.11", "2026.11.1"), Ordering::Less);
        assert_eq!(order("2027.1", "2026.12"), Ordering::Greater);
        assert_eq!(order("2026.10", "2026.10"), Ordering::Equal);
    }

    #[test]
    fn between_names_the_versions_an_upgrade_crosses() {
        let all = revisions();
        let first = &all[0].version;
        let last = &all[all.len() - 1].version;
        let crossed: Vec<String> = between(first, last).into_iter().map(|r| r.version).collect();
        assert_eq!(crossed, all[1..].iter().map(|r| r.version.clone()).collect::<Vec<_>>());
        assert!(between(last, last).is_empty());
    }
}
