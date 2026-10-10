//! `lyrn doctor`: a repository held to the line's standard.
//!
//! Each rule is one lesson the line learnt the hard way - a release that could
//! not run, a lockfile nobody audited, an old alias in a public history - kept
//! as a check that says what is wrong, how to put it right, and, through
//! `--explain`, why the rule exists at all.
//!
//! Two levels, decided by what fixing costs. A failure can be put right now,
//! in a commit, and makes the run exit 1, so `lyrn doctor` works as a gate. A
//! warning is real but needs something a gate cannot ask for: a rewrite of
//! history that is already public, or a setting on the machine rather than in
//! the repository.

mod actions;
mod ai;
mod audit;
mod authors;
mod git;
mod identity;
mod msrv;
mod node;
mod readme;
mod standard;
mod workflows;

use std::error::Error;
use std::path::{Path, PathBuf};

use crate::cli::DoctorArgs;
use crate::repo::Facts;

use self::git::Git;
use self::workflows::Workflow;

/// Where `--explain` sends the reader for the whole page.
const DOCS: &str = "https://lyrn.lacodda.com/reference/doctor/";

/// A rule of the standard, by the name `--explain` takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Rule {
    Standard,
    Readme,
    Mark,
    Actions,
    Msrv,
    Node,
    Audit,
    AiTraces,
    Authors,
    Identity,
}

impl Rule {
    /// Every rule, in the order a report lists them: the files, then what CI
    /// runs, then the history and who writes it.
    pub const ALL: &'static [Rule] = &[
        Rule::Standard,
        Rule::Readme,
        Rule::Mark,
        Rule::Actions,
        Rule::Msrv,
        Rule::Node,
        Rule::Audit,
        Rule::AiTraces,
        Rule::Authors,
        Rule::Identity,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Rule::Standard => "standard",
            Rule::Readme => "readme",
            Rule::Mark => "mark",
            Rule::Actions => "actions",
            Rule::Msrv => "msrv",
            Rule::Node => "node",
            Rule::Audit => "audit",
            Rule::AiTraces => "ai-traces",
            Rule::Authors => "authors",
            Rule::Identity => "identity",
        }
    }

    /// What the rule holds the repository to, in one line.
    pub fn summary(self) -> &'static str {
        match self {
            Rule::Standard => "Every file of the standard for the repository's form is there",
            Rule::Readme => "The README is a shopfront: short blocks, no manual, one title",
            Rule::Mark => "The project wears a mark of its own, not the line's placeholder",
            Rule::Actions => "No GitHub Action or workflow command that GitHub has retired",
            Rule::Msrv => "rust-version is declared, and a CI job builds on it",
            Rule::Node => "pnpm and Node at the majors the line builds with",
            Rule::Audit => "Every lockfile in the repository is read by an audit job",
            Rule::AiTraces => "No coding assistant's files or stamps in the repository or its history",
            Rule::Authors => "Every author in the history is you, or someone .mailmap names",
            Rule::Identity => "This machine commits as its global identity and ignores assistant files",
        }
    }

    /// Why the rule exists - the lesson behind it - and how to satisfy it.
    pub fn lesson(self) -> &'static str {
        match self {
            Rule::Standard => include_str!("lessons/standard.txt"),
            Rule::Readme => include_str!("lessons/readme.txt"),
            Rule::Mark => include_str!("lessons/mark.txt"),
            Rule::Actions => include_str!("lessons/actions.txt"),
            Rule::Msrv => include_str!("lessons/msrv.txt"),
            Rule::Node => include_str!("lessons/node.txt"),
            Rule::Audit => include_str!("lessons/audit.txt"),
            Rule::AiTraces => include_str!("lessons/ai-traces.txt"),
            Rule::Authors => include_str!("lessons/authors.txt"),
            Rule::Identity => include_str!("lessons/identity.txt"),
        }
    }

    fn check(self, repo: &Repo) -> Outcome {
        match self {
            Rule::Standard => standard::check(repo),
            Rule::Readme => readme::check(repo),
            Rule::Mark => standard::check_mark(repo),
            Rule::Actions => actions::check(repo),
            Rule::Msrv => msrv::check(repo),
            Rule::Node => node::check(repo),
            Rule::Audit => audit::check(repo),
            Rule::AiTraces => ai::check(repo),
            Rule::Authors => authors::check(repo),
            Rule::Identity => identity::check(repo),
        }
    }
}

impl std::fmt::Display for Rule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Warn,
    Fail,
}

/// One thing wrong, and how to put it right.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub level: Level,
    pub what: String,
    pub fix: Option<String>,
}

impl Finding {
    pub fn fail(what: impl Into<String>) -> Self {
        Self {
            level: Level::Fail,
            what: what.into(),
            fix: None,
        }
    }

    pub fn warn(what: impl Into<String>) -> Self {
        Self {
            level: Level::Warn,
            what: what.into(),
            fix: None,
        }
    }

    pub fn fix(mut self, fix: impl Into<String>) -> Self {
        self.fix = Some(fix.into());
        self
    }
}

/// What one rule found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Nothing wrong; the text says what was looked at.
    Pass(String),
    /// The rule does not apply here, and the text says why.
    Skip(String),
    /// Something is wrong.
    Found(Vec<Finding>),
}

impl Outcome {
    /// Nothing found is a pass: a rule that looked and found nothing has to
    /// say so rather than report an empty list.
    pub fn from_findings(findings: Vec<Finding>, pass: impl FnOnce() -> String) -> Self {
        if findings.is_empty() {
            Outcome::Pass(pass())
        } else {
            Outcome::Found(findings)
        }
    }

    fn level(&self) -> Option<Level> {
        match self {
            Outcome::Found(findings) => findings.iter().map(|f| f.level).max(),
            _ => None,
        }
    }
}

/// Everything the rules read, gathered once.
pub struct Repo {
    pub root: PathBuf,
    /// What the repository says about itself; a manifest that does not parse
    /// is kept as the error, for the rules that need it to say so.
    pub facts: Result<Facts, String>,
    /// The work tree's git, when the repository is one and git is installed.
    pub git: Option<Git>,
    /// The files of the repository as `/`-separated paths: the tracked ones
    /// in a work tree, else everything outside the build and dependency
    /// directories.
    pub files: Vec<String>,
    pub workflows: Vec<Workflow>,
    /// Workflow files that do not parse, with the parser's reason.
    pub broken_workflows: Vec<(String, String)>,
    /// A CI runner has no identity and no excludes file of its own: the rules
    /// about the machine have nothing to hold it to.
    pub on_ci: bool,
}

impl Repo {
    pub fn read(path: &Path) -> Repo {
        let git = Git::open(path);
        let root = git.as_ref().map(|g| g.root().to_path_buf()).unwrap_or_else(|| path.to_path_buf());
        let files = match &git {
            Some(git) => git.tracked_files(),
            None => walk(&root),
        };
        let (workflows, broken_workflows) = workflows::load(&root, &files);
        Repo {
            facts: Facts::read(&root).map_err(|e| e.to_string()),
            git,
            files,
            workflows,
            broken_workflows,
            on_ci: std::env::var_os("CI").is_some_and(|v| !v.is_empty() && v != "false"),
            root,
        }
    }

    /// The tracked files whose name is `name`, at any depth.
    pub fn files_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a str> + 'a {
        self.files.iter().map(String::as_str).filter(move |path| path.rsplit('/').next() == Some(name))
    }

    pub fn has_file(&self, path: &str) -> bool {
        self.files.iter().any(|f| f == path)
    }

    pub fn read_text(&self, path: &str) -> Option<String> {
        std::fs::read_to_string(self.root.join(path)).ok()
    }
}

/// Every file under `root`, outside version control: what a directory that is
/// not a work tree holds, skipping what a build or an install writes.
fn walk(root: &Path) -> Vec<String> {
    const SKIP: &[&str] = &[".git", "node_modules", "target", "dist", ".astro"];
    fn visit(root: &Path, dir: &Path, out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if path.is_dir() {
                if !SKIP.contains(&name.as_str()) {
                    visit(root, &path, out);
                }
            } else if let Ok(relative) = path.strip_prefix(root) {
                out.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    let mut out = Vec::new();
    visit(root, root, &mut out);
    out.sort();
    out
}

pub fn run(args: DoctorArgs) -> Result<bool, Box<dyn Error>> {
    if let Some(rule) = args.explain {
        explain(rule);
        return Ok(true);
    }

    let path = args.path.clone().unwrap_or_else(|| PathBuf::from("."));
    if !path.is_dir() {
        return Err(format!("`{}` is not a directory - `lyrn doctor` examines a repository", path.display()).into());
    }
    let repo = Repo::read(&path);
    println!("{}\n", heading(&repo));

    let mut counts = Counts::default();
    for &rule in Rule::ALL {
        let outcome = rule.check(&repo);
        print!("{}", render(rule, &outcome));
        counts.add(&outcome);
    }
    println!("\n{}", counts.summary());
    Ok(counts.failed == 0)
}

/// The first line of a report: which repository, and what lyrn takes it for.
fn heading(repo: &Repo) -> String {
    let place = repo.root.display().to_string();
    let Ok(facts) = &repo.facts else {
        return format!("lyrn doctor: {place}");
    };
    let form = facts.form(&repo.root).ok().map(|(form, _)| form);
    match (form, &facts.standard) {
        (Some(form), Some(standard)) => format!("lyrn doctor: {place}, a `{form}` project on standard {standard}"),
        (Some(form), None) => format!("lyrn doctor: {place}, a `{form}` project"),
        (None, _) => format!("lyrn doctor: {place}"),
    }
}

/// One rule's lines of the report.
fn render(rule: Rule, outcome: &Outcome) -> String {
    const COLUMN: usize = 11;
    let name = rule.name();
    match outcome {
        Outcome::Pass(text) => format!("✓ {name:<COLUMN$}{text}\n"),
        Outcome::Skip(text) => format!("- {name:<COLUMN$}{text}\n"),
        Outcome::Found(findings) => {
            let mark = if outcome.level() == Some(Level::Fail) { '✗' } else { '!' };
            let mut out = format!("{mark} {name}\n");
            for finding in findings {
                let sign = match finding.level {
                    Level::Fail => '✗',
                    Level::Warn => '!',
                };
                out.push_str(&format!("    {sign} {}\n", finding.what));
                if let Some(fix) = &finding.fix {
                    for (i, line) in fix.lines().enumerate() {
                        let lead = if i == 0 { "→" } else { " " };
                        out.push_str(&format!("      {lead} {line}\n"));
                    }
                }
            }
            out.push_str(&format!("    why: lyrn doctor --explain {name}\n"));
            out
        }
    }
}

#[derive(Default)]
struct Counts {
    failed: usize,
    warned: usize,
    passed: usize,
    skipped: usize,
}

impl Counts {
    fn add(&mut self, outcome: &Outcome) {
        match (outcome, outcome.level()) {
            (Outcome::Pass(_), _) => self.passed += 1,
            (Outcome::Skip(_), _) => self.skipped += 1,
            (_, Some(Level::Fail)) => self.failed += 1,
            _ => self.warned += 1,
        }
    }

    fn summary(&self) -> String {
        let mut parts = Vec::new();
        if self.failed > 0 {
            parts.push(format!("{} failed", self.failed));
        }
        if self.warned > 0 {
            parts.push(format!("{} to look at", self.warned));
        }
        parts.push(format!("{} passed", self.passed));
        if self.skipped > 0 {
            parts.push(format!("{} not applicable", self.skipped));
        }
        parts.join(", ")
    }
}

fn explain(rule: Rule) {
    println!("{} - {}\n", rule.name(), rule.summary());
    println!("{}", rule.lesson().trim_end());
    println!("\nMore: {DOCS}#{}", rule.name());
}

/// `n thing` or `n things`, for counts in findings.
pub fn plural(count: usize, one: &str, many: &str) -> String {
    if count == 1 { format!("1 {one}") } else { format!("{count} {many}") }
}

/// A short list for a finding: the first few, and how many more.
pub fn some_of(items: &[String], shown: usize) -> String {
    if items.len() <= shown {
        items.join(", ")
    } else {
        format!("{} and {} more", items[..shown].join(", "), items.len() - shown)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `--explain` takes the names the report prints: a rule whose clap name
    /// drifted from its report name would be explained under another word.
    #[test]
    fn every_rule_is_explained_under_the_name_it_is_reported_by() {
        use clap::ValueEnum;
        for rule in Rule::ALL {
            let value = rule.to_possible_value().expect("every rule can be named");
            assert_eq!(value.get_name(), rule.name());
            assert!(!rule.lesson().trim().is_empty(), "{rule} has no lesson");
        }
        assert_eq!(Rule::ALL.len(), Rule::value_variants().len(), "a rule is missing from Rule::ALL");
    }

    #[test]
    fn a_failure_outweighs_a_warning_in_the_report() {
        let outcome = Outcome::Found(vec![Finding::warn("a"), Finding::fail("b").fix("do this\nthen that")]);
        let text = render(Rule::Readme, &outcome);
        assert!(text.starts_with("✗ readme\n"), "{text}");
        assert!(text.contains("    ! a\n"), "{text}");
        assert!(text.contains("    ✗ b\n      → do this\n        then that\n"), "{text}");
        assert!(text.ends_with("    why: lyrn doctor --explain readme\n"), "{text}");

        let warned = Outcome::Found(vec![Finding::warn("a")]);
        assert!(render(Rule::Readme, &warned).starts_with("! readme\n"));
    }

    #[test]
    fn the_summary_counts_each_rule_once() {
        let mut counts = Counts::default();
        counts.add(&Outcome::Found(vec![Finding::warn("a"), Finding::fail("b")]));
        counts.add(&Outcome::Found(vec![Finding::warn("c")]));
        counts.add(&Outcome::Pass("fine".into()));
        counts.add(&Outcome::Skip("n/a".into()));
        assert_eq!(counts.summary(), "1 failed, 1 to look at, 1 passed, 1 not applicable");
    }
}
