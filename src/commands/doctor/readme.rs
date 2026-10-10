//! `readme`: the README is a shopfront, not the manual.
//!
//! What it is, what it looks like at work, how to install it - and the rest
//! on the documentation site, in CONTRIBUTING.md, in the changelog. Measured
//! by blocks rather than by the file: a README of a hundred lines can still
//! hold three twenty-line sections that are pages of the docs site, and the
//! line count never shows it.

use super::{Finding, Outcome, Repo};

/// A block of prose longer than about this is a page of the documentation
/// site: the line's measure is "about 90 words".
pub const BLOCK_WORDS: usize = 90;

/// "About" ends here: past it the block fails, up to it a block over
/// `BLOCK_WORDS` is only worth a look. The line's READMEs were measured to
/// "about 90" by eye, and a gate failing a 92-word paragraph accepted then
/// would be noise rather than a finding.
pub const MAX_BLOCK_WORDS: usize = 100;

/// Status is one paragraph of two to four lines: the version, what works
/// today, how stable it is, the changelog.
pub const STATUS_WORDS: usize = 60;

/// A README this long holds pages of the site whatever its blocks measure.
pub const MAX_LINES: usize = 300;

/// Sections a shopfront does not have, and where their text belongs.
const NOT_HERE: &[(&str, &str)] = &[
    ("roadmap", "the plan lives outside the shopfront, and goes stale inside it"),
    ("plans", "the plan lives outside the shopfront, and goes stale inside it"),
    ("todo", "the plan lives outside the shopfront, and goes stale inside it"),
    ("build", "building it belongs in CONTRIBUTING.md"),
    ("building", "building it belongs in CONTRIBUTING.md"),
    ("development", "developing it belongs in CONTRIBUTING.md"),
    ("developing", "developing it belongs in CONTRIBUTING.md"),
    ("layout", "the layout of the code belongs in CONTRIBUTING.md"),
    ("project layout", "the layout of the code belongs in CONTRIBUTING.md"),
    ("structure", "the layout of the code belongs in CONTRIBUTING.md"),
    ("project structure", "the layout of the code belongs in CONTRIBUTING.md"),
    ("principles", "principles are decisions: docs/adr/ or the docs site"),
    ("design notes", "design notes are decisions: docs/adr/ or the docs site"),
];

pub fn check(repo: &Repo) -> Outcome {
    let Some(text) = repo.read_text("README.md") else {
        return Outcome::Skip("no README.md - `standard` reports it".to_string());
    };
    let findings = examine(&text);
    Outcome::from_findings(findings, || {
        let longest = blocks(&text).iter().filter(|b| b.kind != Kind::Heading).map(|b| b.words).max().unwrap_or(0);
        format!("the longest block is {longest} words, in {} lines", text.lines().count())
    })
}

/// Everything wrong with a README's text.
pub fn examine(text: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let blocks = blocks(text);

    let lines = text.lines().count();
    if lines > MAX_LINES {
        findings.push(
            Finding::fail(format!("{lines} lines: pages of the documentation site are in it"))
                .fix("move the reference to the docs site and link to it; around 150 lines is the mark"),
        );
    }

    for block in blocks.iter().filter(|b| b.kind != Kind::Heading && b.words > BLOCK_WORDS) {
        let place = match &block.section {
            Some(section) => format!("under \"{section}\""),
            None => "at the top".to_string(),
        };
        let what = format!(
            "a {} of {} words {place}, line {}: \"{}\"",
            block.kind.noun(),
            block.words,
            block.line,
            block.opening
        );
        let finding = if block.words > MAX_BLOCK_WORDS {
            Finding::fail(what)
        } else {
            Finding::warn(what)
        };
        findings.push(finding.fix(format!(
            "a block over about {BLOCK_WORDS} words is a page of the docs site: move it there and link, or say it shorter"
        )));
    }

    for block in blocks.iter().filter(|b| b.kind == Kind::Heading && b.level >= 2) {
        let heading = normalize(&block.text);
        if let Some((_, instead)) = NOT_HERE.iter().find(|(name, _)| heading == *name || heading.starts_with(&format!("{name} "))) {
            findings.push(Finding::fail(format!("a \"{}\" section, line {}: {instead}", block.text, block.line)).fix("move it there, then cut it"));
        }
    }

    // The banner carries the name; a title under it says it twice and pushes
    // the promise down. A README without a banner names itself with one.
    let has_banner = text.lines().take_while(|l| !l.starts_with('#')).any(|l| l.contains("<img"));
    if has_banner && let Some(title) = blocks.iter().find(|b| b.kind == Kind::Heading && b.level == 1) {
        findings.push(
            Finding::fail(format!(
                "`# {}` under the banner, line {}: the banner already says the name",
                title.text, title.line
            ))
            .fix("remove the title"),
        );
    }

    let first_section = blocks.iter().position(|b| b.kind == Kind::Heading && b.level >= 2).unwrap_or(blocks.len());
    if !blocks[..first_section].iter().any(|b| b.kind == Kind::Quote) {
        findings
            .push(Finding::warn("no promise before the first section: what it is, in one sentence").fix("say it as a `>` quote under the banner or the title"));
    }

    if let Some(at) = blocks
        .iter()
        .position(|b| b.kind == Kind::Heading && b.level == 2 && normalize(&b.text) == "status")
    {
        let end = blocks[at + 1..]
            .iter()
            .position(|b| b.kind == Kind::Heading && b.level <= 2)
            .map_or(blocks.len(), |i| at + 1 + i);
        let section = &blocks[at + 1..end];
        let prose = section.iter().filter(|b| b.kind == Kind::Prose).count();
        let other = section.iter().any(|b| matches!(b.kind, Kind::Item | Kind::Table));
        let words: usize = section.iter().map(|b| b.words).sum();
        let shape = if other || prose > 1 {
            Some("is more than one paragraph".to_string())
        } else if words > STATUS_WORDS {
            Some(format!("is {words} words, where two to four lines say it"))
        } else {
            None
        };
        if let Some(shape) = shape {
            findings.push(
                Finding::warn(format!("the Status section, line {}, {shape}", blocks[at].line))
                    .fix("one paragraph: the version, what works today, how stable it is, a link to the changelog"),
            );
        }
    }

    findings
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Heading,
    Prose,
    Item,
    Quote,
    Table,
}

impl Kind {
    fn noun(self) -> &'static str {
        match self {
            Kind::Item => "list item",
            Kind::Quote => "quote",
            _ => "paragraph",
        }
    }
}

#[derive(Debug)]
struct Block {
    kind: Kind,
    /// For a heading, its level.
    level: usize,
    /// For a heading, its text.
    text: String,
    words: usize,
    line: usize,
    /// The first few words, to find the block by.
    opening: String,
    /// The `##` section it is in.
    section: Option<String>,
}

/// The README cut into the blocks a reader sees: headings, paragraphs, list
/// items one by one, quotes. Code and HTML are not prose and are left out;
/// tables are kept only so a section knows it holds one.
fn blocks(text: &str) -> Vec<Block> {
    let mut blocks: Vec<Block> = Vec::new();
    let mut fence: Option<&str> = None;
    let mut open = false;
    let mut section: Option<String> = None;

    for (index, raw) in text.lines().enumerate() {
        let line_no = index + 1;
        let line = raw.trim();
        if let Some(marker) = fence {
            if line.starts_with(marker) {
                fence = None;
            }
            continue;
        }
        if line.starts_with("```") || line.starts_with("~~~") {
            fence = Some(&line[..3]);
            open = false;
            continue;
        }
        if line.is_empty() || line.starts_with('<') {
            open = false;
            continue;
        }
        if let Some(level) = heading_level(line) {
            let heading = line[level..].trim().trim_end_matches('#').trim().to_string();
            if level <= 2 {
                section = Some(heading.clone());
            }
            blocks.push(Block {
                kind: Kind::Heading,
                level,
                text: heading,
                words: 0,
                line: line_no,
                opening: String::new(),
                section: section.clone(),
            });
            open = false;
            continue;
        }
        if line.starts_with('|') {
            if !(open && blocks.last().is_some_and(|b| b.kind == Kind::Table)) {
                blocks.push(Block {
                    kind: Kind::Table,
                    level: 0,
                    text: String::new(),
                    words: 0,
                    line: line_no,
                    opening: String::new(),
                    section: section.clone(),
                });
            }
            open = true;
            continue;
        }
        let (kind, content) = if let Some(rest) = list_item(line) {
            (Kind::Item, rest)
        } else if let Some(rest) = line.strip_prefix('>') {
            (Kind::Quote, rest)
        } else {
            (Kind::Prose, line)
        };
        let continues = open
            && blocks.last().is_some_and(|b| match kind {
                // A new item starts a new block; a line under an item belongs to it.
                Kind::Item => false,
                Kind::Quote => b.kind == Kind::Quote,
                _ => matches!(b.kind, Kind::Prose | Kind::Item | Kind::Quote),
            });
        let words = count_words(content);
        if continues {
            let block = blocks.last_mut().expect("checked above");
            block.words += words;
        } else {
            blocks.push(Block {
                kind,
                level: 0,
                text: String::new(),
                words,
                line: line_no,
                opening: opening(content),
                section: section.clone(),
            });
        }
        open = true;
    }
    blocks
}

fn heading_level(line: &str) -> Option<usize> {
    let level = line.chars().take_while(|&c| c == '#').count();
    ((1..=6).contains(&level) && line[level..].starts_with(' ')).then_some(level)
}

fn list_item(line: &str) -> Option<&str> {
    for marker in ["- ", "* ", "+ "] {
        if let Some(rest) = line.strip_prefix(marker) {
            return Some(rest);
        }
    }
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    if digits > 0 {
        let rest = &line[digits..];
        if let Some(rest) = rest.strip_prefix(". ").or_else(|| rest.strip_prefix(") ")) {
            return Some(rest);
        }
    }
    None
}

/// Words a reader reads: a link's text counts, its address does not, and
/// neither do the tags of inline HTML.
fn count_words(text: &str) -> usize {
    readable(text).split_whitespace().filter(|w| w.chars().any(char::is_alphanumeric)).count()
}

fn readable(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find("](") {
        out.push_str(&rest[..at + 1]);
        rest = &rest[at + 2..];
        match rest.find(')') {
            Some(close) => rest = &rest[close + 1..],
            None => rest = "",
        }
    }
    out.push_str(rest);
    // Inline tags: `<kbd>`, `<br>`.
    let mut plain = String::with_capacity(out.len());
    let mut in_tag = false;
    for c in out.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            c if !in_tag => plain.push(c),
            _ => {}
        }
    }
    plain
}

fn opening(text: &str) -> String {
    let plain = readable(text);
    let words: Vec<&str> = plain.split_whitespace().take(8).collect();
    format!("{} ...", words.join(" "))
}

/// A heading as it is compared: lower case, without emoji or punctuation.
fn normalize(heading: &str) -> String {
    let kept: String = heading
        .chars()
        .map(|c| if c.is_alphanumeric() || c == ' ' { c.to_ascii_lowercase() } else { ' ' })
        .collect();
    kept.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::doctor::Level;

    const GOOD: &str = "\
<p align=\"center\"><img src=\"assets/banner.svg\" alt=\"demo\" width=\"720\"></p>

> Does one thing well, in one sentence.

## Why

Short and to the point.

## Install

```console
$ cargo install demo
# a comment in code is not a heading
```

## Status

v1.0, in daily use. See the [CHANGELOG](https://example.com/CHANGELOG.md).

## License

MIT
";

    fn words(n: usize) -> String {
        (0..n).map(|i| format!("word{i}")).collect::<Vec<_>>().join(" ")
    }

    #[test]
    fn a_shopfront_passes() {
        assert_eq!(examine(GOOD), Vec::new());
    }

    #[test]
    fn a_long_paragraph_is_a_page_of_the_docs() {
        let text = GOOD.replace("Short and to the point.", &format!("{}\n{}", words(60), words(41)));
        let findings = examine(&text);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].level, Level::Fail);
        assert!(
            findings[0].what.starts_with("a paragraph of 101 words under \"Why\", line 7"),
            "{}",
            findings[0].what
        );
        // About ninety is worth a look rather than a failure; ninety is fine.
        let edge = examine(&GOOD.replace("Short and to the point.", &words(100)));
        assert_eq!(edge.iter().map(|f| f.level).collect::<Vec<_>>(), vec![Level::Warn], "{edge:?}");
        assert_eq!(examine(&GOOD.replace("Short and to the point.", &words(90))), Vec::new());
    }

    /// Each item of a list is its own block: ten short items are a fine list,
    /// one item of a hundred words is a page.
    #[test]
    fn list_items_are_measured_one_by_one() {
        let list: String = (0..10).map(|_| format!("- {}\n", words(20))).collect();
        assert_eq!(examine(&GOOD.replace("Short and to the point.\n", &list)), Vec::new());
        let long = format!("- short\n- {}\n  {}\n", words(60), words(45));
        let findings = examine(&GOOD.replace("Short and to the point.\n", &long));
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].what.starts_with("a list item of 105 words"), "{}", findings[0].what);
    }

    #[test]
    fn code_tables_and_link_addresses_are_not_words() {
        let table: String = (0..30).map(|_| format!("| {} |\n", words(10))).collect();
        let code = format!("```\n{}\n```\n", words(200));
        let link = format!("[docs]({})", (0..200).map(|i| format!("p{i}")).collect::<Vec<_>>().join("/"));
        let text = GOOD.replace("Short and to the point.\n", &format!("{table}\n{code}\n{link}\n"));
        assert_eq!(examine(&text), Vec::new());
    }

    #[test]
    fn a_section_of_the_manual_is_named_and_sent_away() {
        for heading in [
            "## Development",
            "## 🏗️ Development",
            "### Building from source",
            "## Roadmap",
            "## Project Structure",
        ] {
            let text = GOOD.replace("## Why", heading);
            let findings = examine(&text);
            assert_eq!(findings.len(), 1, "{heading}: {findings:?}");
            assert!(findings[0].what.contains("section"), "{heading}: {}", findings[0].what);
        }
        // A word that merely starts the same is not the section.
        assert_eq!(examine(&GOOD.replace("## Why", "## Buildings it draws")), Vec::new());
    }

    #[test]
    fn a_title_under_the_banner_says_the_name_twice() {
        let findings = examine(&GOOD.replace("> Does one", "# demo\n\n> Does one"));
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].what.starts_with("`# demo` under the banner"));
        // Without a banner, the title is how the README names itself.
        let plain = GOOD.replace("<p align=\"center\"><img src=\"assets/banner.svg\" alt=\"demo\" width=\"720\"></p>", "# demo");
        assert_eq!(examine(&plain), Vec::new());
    }

    #[test]
    fn the_promise_comes_before_the_first_section() {
        let findings = examine(&GOOD.replace("> Does one thing well, in one sentence.", "Does one thing well."));
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].what.starts_with("no promise"));
    }

    #[test]
    fn status_is_one_paragraph() {
        let text = GOOD.replace("v1.0, in daily use.", "v1.0, in daily use.\n\n- works\n- also works\n");
        let findings = examine(&text);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].what.starts_with("the Status section"));
    }

    #[test]
    fn status_says_it_in_two_to_four_lines() {
        let status = "v1.0, in daily use. See the [CHANGELOG](https://example.com/CHANGELOG.md).";
        let findings = examine(&GOOD.replace(status, &words(61)));
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].what.contains("is 61 words"), "{}", findings[0].what);
        assert_eq!(examine(&GOOD.replace(status, &words(60))), Vec::new());
    }

    #[test]
    fn a_long_file_holds_pages_whatever_its_blocks() {
        let text = format!("{GOOD}{}", "short line\n\n".repeat(150));
        let findings = examine(&text);
        assert!(findings.iter().any(|f| f.what.contains("lines: pages")), "{findings:?}");
    }
}
