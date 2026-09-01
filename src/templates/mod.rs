//! The forms lyrn carries built in.
//!
//! A form is a directory of template files plus a manifest, both embedded in
//! the binary so that `lyrn new` works offline.

pub mod spa;

use crate::generate::SourceFile;
use crate::model::Form;

/// The manifest text for a form.
pub fn manifest_for(form: Form) -> &'static str {
    match form {
        Form::Spa => spa::MANIFEST,
    }
}

/// The files a form writes.
pub fn sources_for(form: Form) -> Vec<SourceFile> {
    match form {
        Form::Spa => spa::sources(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    ];

    #[test]
    fn no_form_asks_for_a_variable_nothing_provides() {
        for form in Form::ALL {
            let manifest: TemplateManifest = toml::from_str(manifest_for(*form)).unwrap();
            for source in sources_for(*form) {
                if manifest.verbatim.iter().any(|v| v == source.path) {
                    continue;
                }
                for key in render::placeholders(source.contents).into_iter().chain(render::placeholders(source.path)) {
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
