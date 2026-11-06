//! `lyrn changelog`: a version's section, and the screens that go with it.
//!
//! git-cliff writes a release's section from the commits, and commits say what
//! changed in words. For a product with a screen the words are half of it: the
//! release that redrew the board reads the same as one that renamed a variable.
//! So the screens of a version are kept beside the code, in `docs/screens/<tag>/`
//! - outside what the packages carry - and go into the section under absolute
//!   links at the tag, which hold on every shopfront the changelog is read on.

use std::error::Error;
use std::path::PathBuf;

use crate::cli::ChangelogArgs;
use crate::commands::check::product::Product;
use crate::commands::doctor::Repo;
use crate::commands::doctor::changelog::is_release;

const FILE: &str = "CHANGELOG.md";

/// The heading the screens go under, in the voice of cliff.toml's groups.
const HEADING: &str = "### 🖼️ Screens";

pub fn run(args: ChangelogArgs) -> Result<(), Box<dyn Error>> {
    let root = args.path.clone().unwrap_or_else(|| PathBuf::from("."));
    if !root.is_dir() {
        return Err(format!("`{}` is not a directory - `lyrn changelog` reads a repository", root.display()).into());
    }
    let file = root.join(FILE);
    let text = std::fs::read_to_string(&file).map_err(|e| format!("{FILE}: {e}"))?;
    let section = find_section(&text, args.release.as_deref()).ok_or_else(|| match &args.release {
        Some(version) => format!("{FILE} has no section for {version}"),
        None => format!("{FILE} has no release yet"),
    })?;

    if !args.screens {
        print!("{}", section.text(&text));
        return Ok(());
    }

    let repo = Repo::read(&root);
    let tag = format!("v{}", section.version);
    let shots = screens_of(&repo, &tag);
    if shots.is_empty() {
        return Err(format!("no screens for {tag}: they are kept in docs/screens/{tag}/, as .png, .jpg, .webp or .gif").into());
    }
    let github = Product::read(&repo)
        .github
        .ok_or("the changelog is read away from the repository, so its images need an absolute address, and this repository names none on GitHub")?;
    let block = screens_block(&github, &tag, &shots);
    let written = with_screens(&text, &section, &block);
    if written == text {
        println!("{FILE} already shows the {} of {tag}.", plural(shots.len()));
        return Ok(());
    }
    std::fs::write(&file, &written).map_err(|e| format!("{FILE}: {e}"))?;
    println!("{FILE}: {} under {tag}.", plural(shots.len()));
    println!(
        "The release page keeps the notes it was published with: add them with `gh release edit {tag}` if they belong there too. The images have to be at the tag for the links to answer."
    );
    Ok(())
}

fn plural(count: usize) -> String {
    if count == 1 { "1 screen".to_string() } else { format!("{count} screens") }
}

/// Where a release's section is in the file.
#[derive(Debug, PartialEq, Eq)]
struct Section {
    version: String,
    start: usize,
    end: usize,
}

impl Section {
    fn text<'a>(&self, changelog: &'a str) -> &'a str {
        &changelog[self.start..self.end]
    }
}

/// The section of `version`, or the newest. A heading has the number in
/// brackets, `## 🎉 [2.16.0] - 2026-11-02`, and a tag's `v` is taken off whoever
/// typed it.
fn find_section(text: &str, version: Option<&str>) -> Option<Section> {
    let wanted = version.map(|v| v.trim_start_matches('v'));
    let mut offset = 0;
    let mut found: Option<Section> = None;
    for line in text.split_inclusive('\n') {
        let at = offset;
        offset += line.len();
        if !is_release(line.trim_end_matches(['\n', '\r'])) {
            continue;
        }
        if let Some(section) = &mut found {
            section.end = at;
            return found;
        }
        let number = line.split('[').nth(1).and_then(|rest| rest.split(']').next()).unwrap_or_default();
        if wanted.is_none_or(|w| w == number) {
            found = Some(Section {
                version: number.to_string(),
                start: at,
                end: text.len(),
            });
        }
    }
    found
}

/// The images of a tag, in name order: a number in front of a name decides the
/// order and stays out of the caption.
fn screens_of(repo: &Repo, tag: &str) -> Vec<String> {
    let dir = repo.root.join("docs").join("screens").join(tag);
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|name| {
            let lower = name.to_ascii_lowercase();
            [".png", ".jpg", ".jpeg", ".webp", ".gif"].iter().any(|e| lower.ends_with(e))
        })
        .collect();
    names.sort();
    names
}

/// `02-the-new-board.png` is "The new board".
fn caption(name: &str) -> String {
    let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
    let words = stem.trim_start_matches(|c: char| c.is_ascii_digit() || c == '-' || c == '_');
    let words = if words.is_empty() { stem } else { words };
    let spaced = words.replace(['-', '_'], " ");
    let mut chars = spaced.chars();
    chars.next().map(|first| first.to_uppercase().chain(chars).collect()).unwrap_or_default()
}

fn screens_block(github: &str, tag: &str, names: &[String]) -> String {
    let mut block = format!("{HEADING}\n\n");
    for name in names {
        block.push_str(&format!(
            "![{}](https://raw.githubusercontent.com/{github}/{tag}/docs/screens/{tag}/{name})\n\n",
            caption(name)
        ));
    }
    block
}

/// The changelog with the block at the end of the section, in place of the one
/// there already: a second run with a new image changes it rather than adds a
/// second block.
fn with_screens(text: &str, section: &Section, block: &str) -> String {
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let body = section.text(text).replace("\r\n", "\n");
    let kept = match body.find(HEADING) {
        Some(at) => {
            // The block runs to the next heading of the section, or its end.
            let rest = &body[at + HEADING.len()..];
            let after = rest.find("\n#").map(|n| &rest[n + 1..]).unwrap_or("");
            format!("{}{after}", &body[..at])
        }
        None => body.clone(),
    };
    let new = format!("{}\n\n{block}", kept.trim_end()).replace('\n', newline);
    format!("{}{new}{}", &text[..section.start], &text[section.end..])
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOG: &str = "# Changelog\n\nWords.\n\n## 🎉 [1.2.0] - 2026-11-02\n\n### ✨ Features\n\n- Two\n\n## 🎉 [1.1.0] - 2026-10-02\n\n- One\n";

    #[test]
    fn a_section_runs_to_the_next_release() {
        let newest = find_section(LOG, None).unwrap();
        assert_eq!(newest.version, "1.2.0");
        assert_eq!(newest.text(LOG), "## 🎉 [1.2.0] - 2026-11-02\n\n### ✨ Features\n\n- Two\n\n");
        let older = find_section(LOG, Some("v1.1.0")).unwrap();
        assert_eq!(older.text(LOG), "## 🎉 [1.1.0] - 2026-10-02\n\n- One\n");
        assert_eq!(find_section(LOG, Some("9.9.9")), None);
        assert_eq!(find_section("# Changelog\n", None), None);
    }

    #[test]
    fn a_caption_is_the_name_in_words() {
        assert_eq!(caption("02-the-new-board.png"), "The new board");
        assert_eq!(caption("board.webp"), "Board");
        assert_eq!(caption("01.png"), "01");
    }

    #[test]
    fn the_screens_go_at_the_end_of_the_section_and_are_replaced_there() {
        let section = find_section(LOG, None).unwrap();
        let block = screens_block("acme/app", "v1.2.0", &["01-board.png".to_string()]);
        let once = with_screens(LOG, &section, &block);
        assert!(
            once.contains(
                "- Two\n\n### 🖼️ Screens\n\n![Board](https://raw.githubusercontent.com/acme/app/v1.2.0/docs/screens/v1.2.0/01-board.png)\n\n## 🎉 [1.1.0]"
            ),
            "{once}"
        );

        // A second run with another image changes the block rather than adding one.
        let section = find_section(&once, None).unwrap();
        let block = screens_block("acme/app", "v1.2.0", &["02-list.png".to_string()]);
        let twice = with_screens(&once, &section, &block);
        assert_eq!(twice.matches(HEADING).count(), 1, "{twice}");
        assert!(twice.contains("02-list.png") && !twice.contains("01-board.png"), "{twice}");
        assert!(twice.contains("- Two\n\n### 🖼️ Screens"), "{twice}");
        assert!(twice.contains("## 🎉 [1.1.0]"), "{twice}");
    }
}
