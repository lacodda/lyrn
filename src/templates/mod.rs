//! The forms lyrn carries built in.
//!
//! A form is a directory of template files plus a manifest, both embedded in
//! the binary so that `lyrn new` works offline.

pub mod cli;
pub mod desktop;
pub mod docs;
pub mod dowel;
pub mod mono;
pub mod plugin;
pub mod service;
pub mod spa;
pub mod tauri_plugin;
pub mod workspace;

use crate::generate::{SourceFile, TemplateFile};
use crate::model::Form;

/// The manifest text for a form.
pub fn manifest_for(form: Form) -> &'static str {
    match form {
        Form::Spa => spa::MANIFEST,
        Form::Cli => cli::MANIFEST,
        Form::Desktop => desktop::MANIFEST,
        Form::Service => service::MANIFEST,
        Form::Workspace => workspace::MANIFEST,
        Form::Mono => mono::MANIFEST,
        Form::Plugin => plugin::MANIFEST,
        Form::TauriPlugin => tauri_plugin::MANIFEST,
        Form::Docs => docs::MANIFEST,
    }
}

/// The files a form writes.
pub fn sources_for(form: Form) -> Vec<SourceFile> {
    match form {
        Form::Spa => spa::sources(),
        Form::Cli => cli::sources(),
        Form::Desktop => desktop::sources(),
        Form::Service => service::sources(),
        Form::Workspace => workspace::sources(),
        Form::Mono => mono::sources(),
        Form::Plugin => plugin::sources(),
        Form::TauriPlugin => tauri_plugin::sources(),
        Form::Docs => docs::sources(),
    }
}

/// The files every repository of the line carries whatever its code does:
/// what `lyrn adopt` brings to one that lacks them.
///
/// Hygiene and the gate, and nothing that depends on how the code is laid
/// out. The release contour - `release.yml`, `publish.yml`, the npm wrapper,
/// the installers - is part of the standard too, but its files only work
/// together and against a particular package layout; adding them one by one
/// beside an unknown build would add a workflow that fails on its first tag.
pub const STANDARD: &[&str] = &[
    "README.md",
    "LICENSE",
    "CHANGELOG.md",
    "cliff.toml",
    ".editorconfig",
    ".gitattributes",
    ".gitignore",
    ".github/workflows/ci.yml",
    "docs/adr/0001-record-architecture-decisions.md",
    "docs/adr/README.md",
    "lyrn.toml",
    // Only the Rust forms write one.
    "rustfmt.toml",
];

/// The files of the standard among a template's, as the template writes them.
pub fn standard_files(files: &[TemplateFile]) -> Vec<TemplateFile> {
    files
        .iter()
        .filter(|f| f.addon.is_none() && STANDARD.contains(&f.path.as_str()))
        .cloned()
        .collect()
}

/// Files whose presence means a form must not add to a repository: it already
/// has what the form would bring.
pub fn foreign_sites_for(form: Form) -> &'static [&'static str] {
    match form {
        Form::Docs => docs::FOREIGN_SITES,
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TemplateManifest;

    // Variables, sections, verbatim entries and hooks of every form are held
    // by `template::problems` - the same check `lyrn template check` runs on a
    // template from outside - in `template::tests`.

    /// `adopt` promises the whole standard, so every form it can adopt has to
    /// write all of it - a form that dropped `cliff.toml` would silently
    /// adopt repositories without one.
    #[test]
    fn every_adoptable_form_writes_the_whole_standard() {
        for form in Form::ALL.iter().filter(|f| f.adoptable()) {
            let files: Vec<TemplateFile> = sources_for(*form).into_iter().map(TemplateFile::from).collect();
            let paths: Vec<String> = standard_files(&files).into_iter().map(|f| f.path).collect();
            for entry in STANDARD.iter().filter(|e| **e != "rustfmt.toml") {
                assert!(paths.iter().any(|p| p == entry), "{form} does not write `{entry}`, which `lyrn adopt` promises");
            }
        }
    }

    #[test]
    fn every_form_has_a_manifest_that_parses() {
        for form in Form::ALL {
            let manifest: TemplateManifest = toml::from_str(manifest_for(*form)).unwrap();
            assert!(!manifest.standard.is_empty(), "{form} declares no standard version");
            // A built-in form knows its own form and add-ons in code; saying
            // them again in the manifest would be a second truth.
            assert!(
                manifest.form.is_none() && manifest.addons.is_empty(),
                "{form} repeats in its manifest what the code says"
            );
        }
    }
}
