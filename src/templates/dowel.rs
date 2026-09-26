//! Copies of dowel's registry components, for the forms that draw a UI.
//!
//! A product of the line presses dowel's Button rather than a `<button>` -
//! dowel-ui's lint refuses the raw element outside `components/ui/` - and its
//! sign-in screen is Field, Input, Panel and Alert. Registry components are
//! copied into a product and never imported from the package, so a generated
//! project has to arrive with the copies, and `lyrn new` works offline, so
//! they travel inside the binary.
//!
//! The files under `dowel/ui/` are written by `tools/vendor-dowel.mjs` from the
//! registry catalogue of the dowel-ui version in `dowel/VERSION`, byte for
//! byte. Two gates keep them honest: the tests below refuse a template that
//! pins a different dowel-ui, and the generated project's own
//! `tools/check-registry.mjs` compares every copy with the package it
//! installed - which lyrn's CI runs for each form that carries them.

use crate::generate::{Contents, SourceFile};
use crate::model::Addon;

/// The dowel-ui release the copies were taken from, as `tools/vendor-dowel.mjs`
/// wrote it.
#[cfg(test)]
pub fn version() -> &'static str {
    include_str!("dowel/VERSION").trim()
}

/// One registry component as a template carries it.
#[derive(Debug, Clone, Copy)]
pub struct Primitive {
    /// Where it lands: `components.json` maps dowel's `@ui/` target to
    /// `src/components/ui`.
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

pub const BUTTON: Primitive = Primitive {
    path: "src/components/ui/button.tsx",
    contents: include_str!("dowel/ui/button.tsx"),
};

pub const FIELD: Primitive = Primitive {
    path: "src/components/ui/field.tsx",
    contents: include_str!("dowel/ui/field.tsx"),
};

pub const INPUT: Primitive = Primitive {
    path: "src/components/ui/input.tsx",
    contents: include_str!("dowel/ui/input.tsx"),
};

pub const PANEL: Primitive = Primitive {
    path: "src/components/ui/panel.tsx",
    contents: include_str!("dowel/ui/panel.tsx"),
};

pub const ALERT: Primitive = Primitive {
    path: "src/components/ui/alert.tsx",
    contents: include_str!("dowel/ui/alert.tsx"),
};

/// Every copy the binary carries.
#[cfg(test)]
pub const ALL: &[Primitive] = &[BUTTON, FIELD, INPUT, PANEL, ALERT];

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
                if ALL.iter().any(|p| source.path.ends_with(p.path)) {
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
        for primitive in ALL {
            assert!(primitive.contents.contains("export "), "{} exports nothing", primitive.path);
        }
    }
}
