//! `assets`: each level of the mark sits where its size calls for it, and an
//! `.ico` lists its largest image first.
//!
//! The line's mark comes in three levels - L from 64px up, M from 28 to 63, S
//! at 27 and below - and every product of the line has put the wrong one
//! somewhere at least once: the filled S tile across a whole `.ico`, so the
//! desktop showed a coloured blob; the S tile in a site's header; a site's
//! copy of the mark a version behind the master. Each was found by someone
//! looking at a screen, months later. Here they are found by reading the
//! files: the copies against their masters byte for byte, and each image of
//! an `.ico` by the one pixel that tells the filled tile from the plate.

use super::{Finding, Outcome, Repair, Repo, is_test_material, plural, some_of};
use crate::commands::assets::read_mark;
use crate::mark::ico::{self, Probe};

const L: &str = "assets/logo.svg";
const M: &str = "assets/logo-m.svg";
const S: &str = "assets/logo-s.svg";
const TOUCH: &str = "assets/apple-touch-icon.png";

/// The line's level boundary: at 27px and below only the filled tile reads.
const S_CEILING: u32 = 27;

/// The sizes Windows asks an `.ico` for: 16 and 24 for the title bar, 32 and
/// 48 for the desktop and Explorer, 256 for the task switcher.
const ICO_SIZES: &[u32] = &[16, 24, 32, 48, 256];

pub fn check(repo: &Repo) -> Outcome {
    let icos: Vec<&str> = repo
        .files
        .iter()
        .map(String::as_str)
        .filter(|f| f.to_ascii_lowercase().ends_with(".ico") && !is_test_material(f))
        .collect();
    let has_mark = repo.has_file(L);
    if !has_mark && icos.is_empty() {
        return Outcome::Skip("no mark in assets/ and no .ico".to_string());
    }

    let mut findings = Vec::new();
    let mut looked_at = 0;

    // An exporter of the repository's own is a second drawing of the level
    // rule, and nine products of fifteen got theirs wrong somewhere.
    for script in repo
        .files
        .iter()
        .filter(|f| f.ends_with("export-assets.mjs") && !is_test_material(f) && !in_a_template(f))
    {
        findings.push(
            Finding::warn(format!("{script} draws the mark by a rule of its own"))
                .fix("delete it - `lyrn assets` draws every raster from the masters, by the line's rule"),
        );
    }

    // The three masters. Without them the levels are one drawing at every
    // size, which is the defect itself.
    let levels = has_mark && repo.has_file(M) && repo.has_file(S);
    // The masters drawn, for the repairs that draw an image again; masters
    // that do not draw leave those findings without one.
    let mark = if levels { read_mark(&repo.root).ok() } else { None };
    if has_mark && !levels {
        let missing: Vec<String> = [M, S].iter().filter(|m| !repo.has_file(m)).map(|m| m.to_string()).collect();
        findings.push(
            Finding::fail(format!("the mark has no {}: one drawing at every size", some_of(&missing, 2)))
                .fix("draw the M and S levels beside assets/logo.svg - L from 64px, M from 28 to 63, S at 27 and below"),
        );
    }

    // The copies sites keep of the masters, byte for byte: a header is L, a
    // favicon is S, a touch icon is the one in assets/.
    if levels {
        for path in &repo.files {
            let Some(master) = master_of(path) else { continue };
            looked_at += 1;
            let (Some(theirs), Some(ours)) = (repo.read_bytes(path), repo.read_bytes(master)) else {
                continue;
            };
            if same(&theirs, &ours, path) {
                continue;
            }
            findings.push(
                Finding::fail(format!("{path} is not {master}: {}", what_it_shows(path)))
                    .fix(format!("copy {master} over it - `lyrn assets` writes every copy from the masters"))
                    .repair(Repair::new(path.clone(), ours, format!("copied from {master}"))),
            );
        }
    }

    // Every .ico: readable, largest first, every size Windows asks for, and
    // each image the level its size calls for.
    for path in &icos {
        looked_at += 1;
        let Some(bytes) = repo.read_bytes(path) else { continue };
        let Some(entries) = ico::entries(&bytes) else {
            findings.push(Finding::fail(format!("{path} is not an icon: its directory cannot be read")).fix("export it again"));
            continue;
        };
        let sizes: Vec<u32> = entries.iter().map(|e| e.size).collect();
        let mut sorted = sizes.clone();
        sorted.sort_by(|a, b| b.cmp(a));
        if sizes != sorted {
            findings.push(
                Finding::fail(format!(
                    "{path} lists its images {}: Tauri takes the first as the window's icon",
                    sizes.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(", ")
                ))
                .fix("the largest first")
                .repair(Repair::new(*path, ico::largest_first(&bytes, &entries), "put its largest image first")),
            );
        }
        let missing: Vec<String> = ICO_SIZES.iter().filter(|s| !sizes.contains(s)).map(|s| format!("{s}px")).collect();
        if !missing.is_empty() {
            let mut finding = Finding::warn(format!("{path} has no {}: Windows scales the nearest image instead", some_of(&missing, 5)))
                .fix("`lyrn assets` draws 16, 24, 32, 48, 64, 128 and 256");
            if let Some(mark) = &mark {
                finding = finding.repair(Repair::new(*path, mark.ico(), "drawn again from the masters, every size from its level"));
            }
            findings.push(finding);
        }
        if levels {
            let wrong: Vec<String> = entries
                .iter()
                .filter(|e| !level_reads(e.size, ico::probe(e.bytes(&bytes))))
                .map(|e| format!("{}px", e.size))
                .collect();
            if !wrong.is_empty() {
                let which = if wrong.len() == 1 {
                    format!("the {} image is", wrong[0])
                } else {
                    format!("the {} images are", some_of(&wrong, 4))
                };
                let mut finding = Finding::fail(format!(
                    "{path}: {which} the wrong level of the mark - S at 27px and below, the plated M and L above"
                ))
                .fix("`lyrn assets` draws each size from the level its size calls for");
                if let Some(mark) = &mark {
                    finding = finding.repair(Repair::new(*path, mark.ico(), "drawn again from the masters, every size from its level"));
                }
                findings.push(finding);
            }
        }
    }

    // The rasters drawn large, by the same share of plate: the logo, and
    // every touch icon, which a phone shows at 180px.
    if levels {
        let rasters = repo
            .files
            .iter()
            .filter(|f| f.rsplit('/').next() == Some("apple-touch-icon.png") && !is_test_material(f) && !in_a_template(f))
            .map(|f| (f.as_str(), 180))
            .chain([("assets/logo-512.png", 512)]);
        for (path, size) in rasters {
            let Some(bytes) = repo.read_bytes(path).filter(|_| repo.has_file(path)) else {
                continue;
            };
            looked_at += 1;
            if ico::probe(&bytes) == Probe::Filled {
                let mut finding = Finding::fail(format!("{path} is the filled S tile at {size}px, where the plated L reads"))
                    .fix("`lyrn assets` draws it from assets/logo.svg");
                if let Some(mark) = &mark {
                    finding = finding.repair(Repair::new(path, mark.png(size), format!("drawn again from L at {size}px")));
                }
                findings.push(finding);
            }
        }
    }

    Outcome::from_findings(findings, || {
        if levels {
            format!("three levels in assets/, {} in place", plural(looked_at, "copy or icon", "copies and icons"))
        } else {
            format!("{} in order", plural(icos.len(), "icon", "icons"))
        }
    })
}

/// The master a site's copy is made from, by where the copy lives.
fn master_of(path: &str) -> Option<&'static str> {
    if is_test_material(path) || in_a_template(path) || path.starts_with("assets/") {
        return None;
    }
    let name = path.rsplit('/').next()?;
    match name {
        "favicon.svg" => Some(S),
        // The site's touch icon is a copy; an installable app may compose its
        // own - on a plate, with room for the mask - and is held to its level
        // instead, below.
        "apple-touch-icon.png" if path.starts_with("docs/") => Some(TOUCH),
        "logo.svg" if path.ends_with("src/assets/logo.svg") => Some(L),
        _ => None,
    }
}

/// A template's files belong to the projects it writes, not to this one.
fn in_a_template(path: &str) -> bool {
    path.split('/').any(|p| p == "templates" || p == "template")
}

fn what_it_shows(path: &str) -> &'static str {
    match path.rsplit('/').next() {
        Some("favicon.svg") => "a favicon is the S tile, the one level that reads in a tab",
        Some("logo.svg") => "a site's header is the L mark",
        _ => "the touch icon is drawn once, in assets/",
    }
}

/// Byte for byte - except that an SVG is text, and a checkout may have given
/// it `\r\n` where the master has `\n`.
fn same(theirs: &[u8], ours: &[u8], path: &str) -> bool {
    if path.ends_with(".svg") {
        let strip = |b: &[u8]| -> Vec<u8> { b.iter().copied().filter(|&c| c != b'\r').collect() };
        strip(theirs) == strip(ours)
    } else {
        theirs == ours
    }
}

/// Whether what an image shows reads at its size; an image this cannot judge
/// is not a finding.
fn level_reads(size: u32, probe: Probe) -> bool {
    match probe {
        Probe::Filled => size <= S_CEILING,
        Probe::Plated => size > S_CEILING,
        Probe::Unknown => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_copy_is_held_to_the_master_its_place_calls_for() {
        assert_eq!(master_of("docs/public/favicon.svg"), Some(S));
        assert_eq!(master_of("web/public/favicon.svg"), Some(S));
        assert_eq!(master_of("docs/src/assets/logo.svg"), Some(L));
        assert_eq!(master_of("docs/site/src/assets/logo.svg"), Some(L));
        assert_eq!(master_of("docs/public/apple-touch-icon.png"), Some(TOUCH));
        assert_eq!(master_of("assets/apple-touch-icon.png"), None, "the master itself");
        assert_eq!(master_of("web/public/apple-touch-icon.png"), None, "an app composes its own");
        assert_eq!(master_of("src/templates/spa/public/apple-touch-icon.png"), None);
        assert_eq!(master_of("src/components/logo.svg"), None);
    }

    #[test]
    fn a_level_reads_at_its_size() {
        assert!(level_reads(16, Probe::Filled));
        assert!(!level_reads(32, Probe::Filled));
        assert!(level_reads(48, Probe::Plated));
        assert!(!level_reads(24, Probe::Plated));
        assert!(level_reads(24, Probe::Unknown));
    }

    #[test]
    fn an_svg_copy_is_the_same_whatever_its_line_endings() {
        assert!(same(b"<svg>\r\n</svg>\r\n", b"<svg>\n</svg>\n", "docs/public/favicon.svg"));
        assert!(!same(b"a\r\n", b"a\n", "docs/public/apple-touch-icon.png"));
    }
}
