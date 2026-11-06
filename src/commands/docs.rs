//! `lyrn docs`: a page for a command, a guide or a concept; the site held to the
//! program; the site served.
//!
//! The line's rule is that a command without a page does not exist, and a rule
//! with no tool behind it is kept by remembering. `ref` writes the page a
//! command needs from the program's own `--help`, so the first draft is right
//! about every flag. `check` says what the program has and the site does not
//! say: a command, a subcommand or a flag with no page, a page for a command
//! that is gone, a link to nothing. `lyrn check` keeps its lighter version of
//! the same promise on every push; this one is for sitting down to the docs.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command as Process;

use regex::Regex;

use crate::cli::DocsCommand;
use crate::commands::check::commands::{self, Help, binary};
use crate::commands::check::product::Product;
use crate::commands::check::site::Site;
use crate::commands::doctor::{Finding, Level, Outcome, Repo, plural};
use crate::naming;

pub fn run(command: DocsCommand) -> Result<bool, Box<dyn Error>> {
    match command {
        DocsCommand::Ref { command, path } => reference(command.as_deref(), path).map(|()| true),
        DocsCommand::Guide { title, description, path } => page("guides", &title, description.as_deref(), path).map(|()| true),
        DocsCommand::Concept { title, description, path } => page("concepts", &title, description.as_deref(), path).map(|()| true),
        DocsCommand::Check { path } => check(path),
        DocsCommand::Dev { path } => dev(path),
    }
}

fn repo_at(path: Option<PathBuf>) -> Result<Repo, Box<dyn Error>> {
    let path = path.unwrap_or_else(|| PathBuf::from("."));
    if !path.is_dir() {
        return Err(format!("`{}` is not a directory - `lyrn docs` works in a repository", path.display()).into());
    }
    // What git tracks says where the repository's top is, and the pages are read
    // as they stand: `docs ref` then `docs check`, with nothing committed between,
    // has to see the page it just wrote.
    Ok(Repo::on_disk(&Repo::read(&path).root))
}

fn site_of(repo: &Repo) -> Result<Site, Box<dyn Error>> {
    Site::read(repo).ok_or_else(|| "this repository has no documentation site - `lyrn new <name> --form docs` adds one".into())
}

fn content_dir(site: &Site) -> String {
    format!("{}/src/content/docs", site.dir)
}

/// A page under `dir`, refusing to write over one: a page someone has written
/// is worth more than a first draft.
fn write_page(root: &Path, path: &str, text: &str) -> Result<(), Box<dyn Error>> {
    let file = root.join(path);
    if file.exists() {
        return Err(format!("{path} is already there").into());
    }
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&file, text).map_err(|e| format!("{path}: {e}"))?;
    println!("Wrote {path}.");
    Ok(())
}

// --- ref -------------------------------------------------------------------

/// A page for the named command, or for every command with no page yet.
fn reference(command: Option<&str>, path: Option<PathBuf>) -> Result<(), Box<dyn Error>> {
    let repo = repo_at(path)?;
    let site = site_of(&repo)?;
    let product = Product::read(&repo);
    let programs = product.programs();
    if programs.is_empty() {
        return Err("`lyrn docs ref` reads a command-line program's `--help`, and this repository builds none".into());
    }
    let dir = site.reference_dir();
    let mut wrote = 0;
    let mut known: Vec<String> = Vec::new();
    for (manifest, bin) in programs {
        let program = binary(&repo, manifest, bin).map_err(|reason| format!("`{bin} --help` could not be read: {reason}"))?;
        let mut help = Help::new(&program);
        for name in help.names(&[]) {
            let wanted = command.is_none_or(|c| c == name);
            known.push(name.clone());
            if !wanted {
                continue;
            }
            let file = format!("{dir}/{name}.md");
            if command.is_none() && (repo.root.join(&file).exists() || repo.root.join(format!("{dir}/{name}.mdx")).exists()) {
                continue;
            }
            write_page(&repo.root, &file, &command_page(&mut help, bin, &name))?;
            wrote += 1;
        }
    }
    match command {
        Some(command) if wrote == 0 => Err(format!("the program has no command `{command}` (it has: {})", known.join(", ")).into()),
        None if wrote == 0 => {
            println!("Every command has a page under {dir}/.");
            Ok(())
        }
        _ => Ok(()),
    }
}

/// A command's page, from its help and its subcommands'.
fn command_page(help: &mut Help, bin: &str, command: &str) -> String {
    let text = help.text(&[command]).unwrap_or_default();
    let about = commands::about(&text);
    let description = if about.ends_with(['.', '!', '?']) || about.is_empty() {
        about.clone()
    } else {
        format!("{about}.")
    };
    let mut page = format!(
        "---\ntitle: {bin} {command}\ndescription: {}\n---\n\n{}\n{description}\n",
        naming::json_string(&description),
        usage_block(&text)
    );
    page.push_str(&tables(&text));
    let subcommands = help.names(&[command]);
    for sub in &subcommands {
        let inner = help.text(&[command, sub]).unwrap_or_default();
        page.push_str(&format!("\n## {bin} {command} {sub}\n\n{}\n{}\n", usage_block(&inner), commands::about(&inner)));
        page.push_str(&tables(&inner));
    }
    page
}

fn usage_block(help: &str) -> String {
    format!("```console\n$ {}\n```\n", commands::usage(help).unwrap_or_default())
}

/// The arguments and options of one help text, as tables.
fn tables(help: &str) -> String {
    let mut out = String::new();
    for (heading, entries) in [("Arguments", commands::arguments(help)), ("Options", commands::options(help))] {
        let entries: Vec<_> = entries
            .into_iter()
            .filter(|(spec, _)| !spec.contains("--help") && !spec.contains("--version"))
            .collect();
        if entries.is_empty() {
            continue;
        }
        out.push_str(&format!("\n| {} | What it does |\n| --- | --- |\n", heading.trim_end_matches('s')));
        for (spec, about) in entries {
            out.push_str(&format!("| `{spec}` | {} |\n", about.replace('|', "\\|")));
        }
    }
    out
}

// --- guide, concept --------------------------------------------------------

fn page(dir: &str, title: &str, description: Option<&str>, path: Option<PathBuf>) -> Result<(), Box<dyn Error>> {
    let repo = repo_at(path)?;
    let site = site_of(&repo)?;
    let title = naming::one_line(title);
    let slug = naming::slug(&title);
    if slug.is_empty() {
        return Err(format!("`{title}` leaves nothing for a file's name: write the title in English").into());
    }
    let file = format!("{}/{dir}/{slug}.md", content_dir(&site));
    let mut front = format!("---\ntitle: {}\n", naming::json_string(&title));
    match description {
        Some(text) => front.push_str(&format!("description: {}\n", naming::json_string(&naming::one_line(text)))),
        None => println!("It has no description yet: `--description` gives one, and the page's frontmatter holds it."),
    }
    front.push_str("---\n\n");
    write_page(&repo.root, &file, &front)?;
    if let Some(config) = site_config(&repo, &site)
        && config.contains("sidebar:")
        && !listed_in(&config, dir)
    {
        println!(
            "The sidebar of {}/astro.config.mjs lists no `{dir}/`: `{{ label: '{}', items: [{{ autogenerate: {{ directory: '{dir}' }} }}] }}` shows it.",
            site.dir,
            capitalise(dir)
        );
    }
    Ok(())
}

fn capitalise(word: &str) -> String {
    let mut chars = word.chars();
    chars.next().map(|first| first.to_uppercase().chain(chars).collect()).unwrap_or_default()
}

fn site_config(repo: &Repo, site: &Site) -> Option<String> {
    repo.read_text(&format!("{}/astro.config.mjs", site.dir))
}

/// Whether a sidebar names a directory of pages, or a page in it.
fn listed_in(config: &str, dir: &str) -> bool {
    let named = Regex::new(&format!(r#"['"]{}(/[^'"]*)?['"]"#, regex::escape(dir))).expect("a valid pattern");
    named.is_match(config)
}

// --- check -----------------------------------------------------------------

fn check(path: Option<PathBuf>) -> Result<bool, Box<dyn Error>> {
    let repo = repo_at(path)?;
    let product = Product::read(&repo);
    let site = site_of(&repo)?;
    println!("lyrn docs check: {}, the site in {}/\n", repo.root.display(), site.dir);
    let pages = Pages::read(&repo, &site);

    let mut outcomes: Vec<(&str, Outcome)> = vec![
        ("commands", commands::check(&repo, &product)),
        ("flags", flags(&repo, &product, &site, &pages)),
        ("pages", page_rules(&repo, &product, &site, &pages)),
        ("links", links(&repo, &site, &pages)),
    ];
    if let Some(config) = site_config(&repo, &site) {
        outcomes.push(("sidebar", sidebar(&config, &pages)));
    }

    let (mut failed, mut warned, mut passed, mut skipped) = (0, 0, 0, 0);
    for (name, outcome) in &outcomes {
        match outcome {
            Outcome::Pass(text) => {
                passed += 1;
                println!("✓ {name:<10}{text}");
            }
            Outcome::Skip(text) => {
                skipped += 1;
                println!("- {name:<10}{text}");
            }
            Outcome::Found(findings) => {
                let fail = outcome.level() == Some(Level::Fail);
                if fail {
                    failed += 1;
                } else {
                    warned += 1;
                }
                println!("{} {name}", if fail { '✗' } else { '!' });
                for finding in findings {
                    println!("    {} {}", if finding.level == Level::Fail { '✗' } else { '!' }, finding.what);
                    if let Some(fix) = &finding.fix {
                        println!("      → {fix}");
                    }
                }
            }
        }
    }
    let mut parts = Vec::new();
    if failed > 0 {
        parts.push(format!("{failed} failed"));
    }
    if warned > 0 {
        parts.push(format!("{warned} to look at"));
    }
    parts.push(format!("{passed} passed"));
    if skipped > 0 {
        parts.push(format!("{skipped} not applicable"));
    }
    println!("\n{}", parts.join(", "));
    Ok(failed == 0)
}

/// The site's pages: path, the address it is served at, and its text.
struct Pages {
    pages: Vec<(String, String, String)>,
}

impl Pages {
    fn read(repo: &Repo, site: &Site) -> Pages {
        let content = format!("{}/", content_dir(site));
        let mut pages = Vec::new();
        for file in repo
            .files
            .iter()
            .filter(|f| f.starts_with(&content) && (f.ends_with(".md") || f.ends_with(".mdx")))
        {
            let Some(text) = repo.read_text(file) else { continue };
            let rest = &file[content.len()..];
            let stem = rest.rsplit_once('.').map_or(rest, |(stem, _)| stem);
            let address = frontmatter(&text, "slug").unwrap_or_else(|| stem.strip_suffix("/index").unwrap_or(stem).to_string());
            let address = if address == "index" { String::new() } else { address };
            pages.push((file.clone(), address.trim_matches('/').to_string(), text));
        }
        Pages { pages }
    }

    fn by_address(&self, address: &str) -> Option<&(String, String, String)> {
        self.pages.iter().find(|(_, a, _)| a == address)
    }
}

/// A key of a page's frontmatter, its quotes off.
fn frontmatter(text: &str, key: &str) -> Option<String> {
    let mut lines = text.lines();
    if lines.next()?.trim_end() != "---" {
        return None;
    }
    for line in lines {
        if line.trim_end() == "---" {
            return None;
        }
        if let Some(value) = line.strip_prefix(&format!("{key}:")) {
            let value = value.trim();
            return Some(value.trim_matches(['"', '\'']).to_string());
        }
    }
    None
}

/// Every flag of every command is on that command's page.
fn flags(repo: &Repo, product: &Product, site: &Site, pages: &Pages) -> Outcome {
    let programs = product.programs();
    if programs.is_empty() {
        return Outcome::Skip("not a command-line program".to_string());
    }
    let reference = format!("{}/", site.reference_dir());
    let mut findings = Vec::new();
    let mut counted = 0;
    for (manifest, bin) in programs {
        let Ok(program) = binary(repo, manifest, bin) else {
            continue;
        };
        let mut help = Help::new(&program);
        for command in help.names(&[]) {
            let text: Option<&str> = pages
                .pages
                .iter()
                .find(|(file, ..)| {
                    file.strip_prefix(&reference)
                        .and_then(|rest| rest.rsplit_once('.'))
                        .is_some_and(|(stem, _)| stem == command || stem == format!("{command}/index"))
                })
                .map(|(.., text)| text.as_str());
            // A command with no page of its own is `commands`' finding, or is
            // walked through on a guide: there is no page to hold it to.
            let Some(text) = text else { continue };
            let mut paths: Vec<Vec<String>> = vec![vec![command.clone()]];
            let words: Vec<&str> = vec![command.as_str()];
            for sub in help.names(&words) {
                paths.push(vec![command.clone(), sub]);
            }
            for path in paths {
                let words: Vec<&str> = path.iter().map(String::as_str).collect();
                let Some(help_text) = help.text(&words) else { continue };
                if path.len() == 2 && !text.contains(&format!("{bin} {}", words.join(" "))) {
                    findings.push(
                        Finding::fail(format!("`{bin} {}` is not on the page of `{bin} {}`", words.join(" "), words[0]))
                            .fix("a command without a page does not exist: name it there"),
                    );
                }
                for (spec, _) in commands::options(&help_text) {
                    let Some(flag) = commands::long_flag(&spec) else { continue };
                    if flag == "--help" || flag == "--version" {
                        continue;
                    }
                    counted += 1;
                    if !mentions_flag(text, &flag) {
                        findings.push(
                            Finding::fail(format!("`{bin} {} {flag}` is not on the command's page", words.join(" "))).fix("say what it does on that page"),
                        );
                    }
                }
            }
        }
    }
    Outcome::from_findings(findings, || format!("{} on their commands' pages", plural(counted, "flag", "flags")))
}

/// Whether a page names a flag whole: `--with` is not in `--without`.
fn mentions_flag(text: &str, flag: &str) -> bool {
    text.match_indices(flag).any(|(at, _)| {
        let next = text[at + flag.len()..].chars().next();
        !next.is_some_and(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    })
}

/// A page has a title and a description, and a page titled for a command is for
/// one the program has.
fn page_rules(repo: &Repo, product: &Product, site: &Site, pages: &Pages) -> Outcome {
    let mut findings = Vec::new();
    let reference = format!("{}/", site.reference_dir());
    let mut known: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    for (manifest, bin) in product.programs() {
        if let Ok(program) = binary(repo, manifest, bin) {
            known.insert(bin, Help::new(&program).names(&[]).into_iter().collect());
        }
    }
    for (file, _, text) in &pages.pages {
        let Some(title) = frontmatter(text, "title") else {
            findings.push(Finding::fail(format!("{file} has no title in its frontmatter")).fix("Starlight will not build a page without one"));
            continue;
        };
        if frontmatter(text, "description").is_none() {
            findings.push(Finding::warn(format!("{file} has no description")).fix("a line for search results and the page's metadata"));
        }
        if file.starts_with(&reference) {
            let mut words = title.split_whitespace();
            if let (Some(bin), Some(command)) = (words.next(), words.next())
                && let Some(commands) = known.get(bin)
                && !commands.contains(command)
            {
                findings.push(
                    Finding::fail(format!("{file} is the page of `{bin} {command}`, which `{bin} --help` does not list"))
                        .fix("remove the page, or give the program the command"),
                );
            }
        }
    }
    Outcome::from_findings(findings, || {
        format!("{} with a title and a description", plural(pages.pages.len(), "page", "pages"))
    })
}

/// What a heading is linked by, as Starlight's slugger writes it.
fn slug_of(heading: &str) -> String {
    let text: String = heading
        .trim()
        .chars()
        .filter(|c| !matches!(c, '`' | '*' | '[' | ']'))
        .flat_map(char::to_lowercase)
        .filter_map(|c| match c {
            ' ' => Some('-'),
            c if c.is_alphanumeric() || c == '-' || c == '_' => Some(c),
            _ => None,
        })
        .collect();
    text
}

/// The anchors of a page: its headings, the second of a name numbered as the
/// slugger numbers it, and any `id` its markup sets.
fn anchors(text: &str) -> BTreeSet<String> {
    let id = Regex::new(r#"\bid=["']([^"']+)["']"#).expect("a valid pattern");
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    let mut out = BTreeSet::new();
    let mut fenced = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        if let Some(rest) = line.strip_prefix('#') {
            let heading = rest.trim_start_matches('#');
            if heading.starts_with(' ') {
                let slug = slug_of(heading);
                let count = seen.entry(slug.clone()).or_default();
                out.insert(if *count == 0 { slug.clone() } else { format!("{slug}-{count}") });
                *count += 1;
            }
        }
        for found in id.captures_iter(line) {
            out.insert(found[1].to_string());
        }
    }
    out
}

/// Every link inside the site reaches a page, and the heading it names.
fn links(repo: &Repo, site: &Site, pages: &Pages) -> Outcome {
    let link = Regex::new(r"\]\(([^)\s]+)\)").expect("a valid pattern");
    let base = site.base.as_deref().unwrap_or("").trim_end_matches('/');
    let mut findings = Vec::new();
    let mut counted = 0;
    for (file, address, text) in &pages.pages {
        let mut fenced = false;
        for (index, line) in text.lines().enumerate() {
            if line.trim_start().starts_with("```") {
                fenced = !fenced;
                continue;
            }
            if fenced {
                continue;
            }
            for found in link.captures_iter(line) {
                let target = &found[1];
                let (target_path, anchor) = target.split_once('#').map_or((target, None), |(p, a)| (p, Some(a)));
                let to_page = if target_path.is_empty() {
                    address.clone()
                } else if let Some(path) = target_path.strip_prefix('/') {
                    let path = format!("/{path}");
                    let path = path.strip_prefix(base).unwrap_or(&path);
                    // A file the site serves as it is.
                    if path.rsplit('/').next().is_some_and(|last| last.contains('.')) {
                        counted += 1;
                        if !repo.has_file(&format!("{}/public{path}", site.dir))
                            && !repo.root.join(&site.dir).join("public").join(path.trim_start_matches('/')).exists()
                        {
                            findings.push(Finding::fail(format!("{file}:{}: {target} is no file of the site", index + 1)).fix("link to what `public/` holds"));
                        }
                        continue;
                    }
                    path.trim_matches('/').to_string()
                } else {
                    continue;
                };
                counted += 1;
                let Some((.., target_text)) = pages.by_address(&to_page) else {
                    findings.push(Finding::fail(format!("{file}:{}: {target} is no page of the site", index + 1)).fix("link to where the page is now"));
                    continue;
                };
                if let Some(anchor) = anchor
                    && !anchors(target_text).contains(anchor)
                {
                    findings.push(Finding::fail(format!("{file}:{}: {target} names a heading the page does not have", index + 1)).fix("link to one it has"));
                }
            }
        }
    }
    Outcome::from_findings(findings, || format!("{} inside the site, all answering", plural(counted, "link", "links")))
}

/// A directory of pages the sidebar lists nothing of cannot be reached by reading.
fn sidebar(config: &str, pages: &Pages) -> Outcome {
    if !config.contains("sidebar:") {
        return Outcome::Skip("no sidebar set: Starlight lists every page".to_string());
    }
    let mut dirs: BTreeSet<&str> = BTreeSet::new();
    for (_, address, _) in &pages.pages {
        if let Some((dir, _)) = address.split_once('/') {
            dirs.insert(dir);
        }
    }
    let findings: Vec<Finding> = dirs
        .iter()
        .filter(|dir| !listed_in(config, dir))
        .map(|dir| {
            Finding::warn(format!("no page of {dir}/ is in the sidebar")).fix(format!(
                "`{{ label: '{}', items: [{{ autogenerate: {{ directory: '{dir}' }} }}] }}` in astro.config.mjs",
                capitalise(dir)
            ))
        })
        .collect();
    Outcome::from_findings(findings, || format!("{} listed", plural(dirs.len(), "directory", "directories")))
}

// --- dev -------------------------------------------------------------------

/// `pnpm dev` where the site lives, installing it first when it has not been:
/// the commands are the site's own, and only where to run them is lyrn's to know.
fn dev(path: Option<PathBuf>) -> Result<bool, Box<dyn Error>> {
    let repo = repo_at(path)?;
    let site = site_of(&repo)?;
    let dir = repo.root.join(&site.dir);
    if !dir.join("node_modules").is_dir() {
        println!("Installing the site's dependencies in {}/...", site.dir);
        if !pnpm(&dir, "install")? {
            return Err(format!("`pnpm install` failed in {}/", site.dir).into());
        }
    }
    pnpm(&dir, "dev")
}

/// `pnpm <command>` in a directory, on the terminal. Through `cmd` on Windows,
/// where pnpm may be a `.cmd` a program cannot start by itself.
fn pnpm(dir: &Path, command: &str) -> Result<bool, Box<dyn Error>> {
    let mut process = if cfg!(windows) {
        let mut process = Process::new("cmd");
        process.args(["/C", "pnpm"]);
        process
    } else {
        Process::new("pnpm")
    };
    let status = process
        .arg(command)
        .current_dir(dir)
        .status()
        .map_err(|e| format!("pnpm did not start: {e} - the line builds its sites with pnpm"))?;
    Ok(status.success())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_heading_is_linked_by_what_the_slugger_writes() {
        assert_eq!(slug_of(" The stack"), "the-stack");
        assert_eq!(slug_of(" `lyrn new`, in detail"), "lyrn-new-in-detail");
        let page = "---\ntitle: x\n---\n\n## Same\n\n```console\n# not a heading\n```\n\n## Same\n\n<a id=\"custom\"></a>\n";
        let found = anchors(page);
        assert!(found.contains("same") && found.contains("same-1") && found.contains("custom"), "{found:?}");
        assert!(!found.contains("not-a-heading"), "{found:?}");
    }

    #[test]
    fn a_flag_is_named_whole() {
        assert!(mentions_flag("`--with x`", "--with"));
        assert!(mentions_flag("use --with", "--with"));
        assert!(!mentions_flag("--without x", "--with"));
        assert!(!mentions_flag("--with-more", "--with"));
    }

    #[test]
    fn the_frontmatter_is_read_with_its_quotes_off() {
        let page = "---\ntitle: \"A title\"\ndescription: Plain\n---\n\ntitle: not this\n";
        assert_eq!(frontmatter(page, "title").as_deref(), Some("A title"));
        assert_eq!(frontmatter(page, "description").as_deref(), Some("Plain"));
        assert_eq!(frontmatter(page, "slug"), None);
        assert_eq!(frontmatter("# no frontmatter", "title"), None);
    }

    #[test]
    fn a_sidebar_lists_a_directory_or_a_page_in_it() {
        let config = "sidebar: [{ items: [{ autogenerate: { directory: 'concepts' } }] }, { label: 'x', slug: 'guides/first' }]";
        assert!(listed_in(config, "concepts"));
        assert!(listed_in(config, "guides"));
        assert!(!listed_in(config, "reference"));
    }
}
