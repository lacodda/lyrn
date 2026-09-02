//! The forms lyrn carries built in.
//!
//! A form is a directory of template files plus a manifest, both embedded in
//! the binary so that `lyrn new` works offline.

pub mod cli;
pub mod desktop;
pub mod spa;

use crate::generate::SourceFile;
use crate::model::Form;

/// The manifest text for a form.
pub fn manifest_for(form: Form) -> &'static str {
    match form {
        Form::Spa => spa::MANIFEST,
        Form::Cli => cli::MANIFEST,
        Form::Desktop => desktop::MANIFEST,
    }
}

/// The files a form writes.
pub fn sources_for(form: Form) -> Vec<SourceFile> {
    match form {
        Form::Spa => spa::sources(),
        Form::Cli => cli::sources(),
        Form::Desktop => desktop::sources(),
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
                        assert!(
                            form.addons().iter().any(|a| a.as_str() == name),
                            "{form}: {} names the section `{name}`, which is not one of its add-ons",
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
