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

use crate::generate::SourceFile;
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

/// The files of a form's standard, as the form writes them.
pub fn standard_sources(form: Form) -> Vec<SourceFile> {
    sources_for(form)
        .into_iter()
        .filter(|s| s.addon.is_none() && STANDARD.contains(&s.path))
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
    use crate::generate::Contents;
    use crate::model::TemplateManifest;
    use crate::render;

    /// The variables `lyrn new` is able to fill in. A template asking for
    /// anything else would only fail once someone ran it.
    const PROVIDED: &[&str] = &[
        "name",
        "title",
        "description",
        "accent",
        "mark",
        "author",
        "form",
        "year",
        "date",
        "registry",
        "lyrn_version",
        "standard",
        "msrv",
        "repo",
        "env_prefix",
        "lib_name",
        "owner",
        "repo_name",
        // The same values, as JSON strings: a description with an apostrophe
        // or a colon is valid text and broken JavaScript or YAML when pasted
        // in bare, and a JSON string is valid in both.
        "title_json",
        "description_json",
        // Only the plugin forms fill these; the check below asks each form for
        // its own, so a variable no form of this shape provides is still an
        // error rather than a name on a list.
        "host",
        "prefix",
        "host_about",
        "host_lookup",
        "protocol_version",
        "subject_about",
        "target",
        "target_about",
        "target_list",
        "command_key",
        "command_label",
        "command_fn",
        "command_camel",
        "type_name",
        "bin_name",
    ];

    #[test]
    fn no_form_asks_for_a_variable_nothing_provides() {
        for form in Form::ALL {
            let manifest: TemplateManifest = toml::from_str(manifest_for(*form)).unwrap();
            for source in sources_for(*form) {
                if manifest.verbatim.iter().any(|v| v == source.path) {
                    continue;
                }
                // Binary files carry no placeholders to check.
                let Contents::Text(text) = source.contents else { continue };
                for key in render::placeholders(text).into_iter().chain(render::placeholders(source.path)) {
                    assert!(
                        PROVIDED.contains(&key.as_str()),
                        "{form}: `{}` asks for `{{{{ {key} }}}}`, which the generator does not provide",
                        source.path
                    );
                }
            }
        }
    }

    #[test]
    fn every_section_marker_is_closed() {
        // An unclosed `{{#addon}}` is silent damage: with the add-on off it
        // swallows the rest of the file, and with it on it changes nothing -
        // so the only person who sees the loss is whoever enables it later.
        for form in Form::ALL {
            for source in sources_for(*form) {
                // Binary files have no sections to balance.
                let Contents::Text(text) = source.contents else { continue };
                let mut open: Option<&str> = None;
                for (number, line) in text.lines().enumerate() {
                    let trimmed = line.trim();
                    let opener = trimmed
                        .strip_prefix("{{#")
                        .or_else(|| trimmed.strip_prefix("{{^"))
                        .and_then(|r| r.strip_suffix("}}"));
                    if let Some(name) = opener {
                        assert!(
                            open.is_none(),
                            "{form}: {} line {}: `{{{{#{name}}}}}` opens inside `{}`, which is still open",
                            source.path,
                            number + 1,
                            open.unwrap()
                        );
                        // Any form's add-on, not only this one's: the desktop and
                        // service forms carry files of the spa form, whose
                        // sections they can never enable and so always cut. A
                        // name no form knows is still a misspelling.
                        assert!(
                            Form::ALL.iter().flat_map(|f| f.addons()).any(|a| a.as_str() == name),
                            "{form}: {} names the section `{name}`, which is not an add-on of any form",
                            source.path
                        );
                        open = Some(name);
                    } else if let Some(name) = trimmed.strip_prefix("{{/").and_then(|r| r.strip_suffix("}}")) {
                        assert_eq!(
                            open,
                            Some(name),
                            "{form}: {} line {}: `{{{{/{name}}}}}` closes a section that is not open",
                            source.path,
                            number + 1
                        );
                        open = None;
                    }
                }
                assert!(open.is_none(), "{form}: {} leaves `{}` unclosed", source.path, open.unwrap_or(""));
            }
        }
    }

    /// `adopt` promises the whole standard, so every form it can adopt has to
    /// write all of it - a form that dropped `cliff.toml` would silently
    /// adopt repositories without one.
    #[test]
    fn every_adoptable_form_writes_the_whole_standard() {
        for form in Form::ALL.iter().filter(|f| f.adoptable()) {
            let paths: Vec<&str> = standard_sources(*form).iter().map(|s| s.path).collect();
            for entry in STANDARD.iter().filter(|e| **e != "rustfmt.toml") {
                assert!(paths.contains(entry), "{form} does not write `{entry}`, which `lyrn adopt` promises");
            }
        }
    }

    #[test]
    fn every_form_has_a_manifest_that_parses() {
        for form in Form::ALL {
            let manifest: TemplateManifest = toml::from_str(manifest_for(*form)).unwrap();
            assert!(!manifest.standard.is_empty(), "{form} declares no standard version");
        }
    }

    #[test]
    fn every_verbatim_entry_names_a_file_the_form_writes() {
        // A stale entry would silently stop protecting anything.
        for form in Form::ALL {
            let manifest: TemplateManifest = toml::from_str(manifest_for(*form)).unwrap();
            let paths: Vec<&str> = sources_for(*form).iter().map(|s| s.path).collect();
            for entry in &manifest.verbatim {
                assert!(paths.contains(&entry.as_str()), "{form}: verbatim names `{entry}`, which it does not write");
            }
        }
    }
}
