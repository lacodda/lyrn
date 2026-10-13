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
//!
//! Some findings carry their own repair: the bytes a file should hold. The
//! rule that finds the problem computes it, so `--fix` only writes what the
//! report already promised, and the two can never disagree.

mod actions;
mod ai;
mod assets;
mod audit;
mod authors;
mod git;
mod ico;
mod identity;
mod language;
mod msrv;
mod node;
mod private;
mod readme;
mod registry;
mod report;
mod stack;
mod standard;
mod workflows;

use std::cell::OnceCell;
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
    Assets,
    Language,
    Private,
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
        Rule::Assets,
        Rule::Language,
        Rule::Private,
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
            Rule::Assets => "assets",
            Rule::Language => "language",
            Rule::Private => "private",
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
            Rule::Assets => "Each level of the mark sits where its size calls for it, and an .ico lists its largest image first",
            Rule::Language => "What ships is written in English; another language lives where its translations go",
            Rule::Private => "Nothing from this computer - its user's home, its other projects, its name - in the repository",
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
            Rule::Assets => include_str!("lessons/assets.txt"),
            Rule::Language => include_str!("lessons/language.txt"),
            Rule::Private => include_str!("lessons/private.txt"),
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
            Rule::Assets => assets::check(repo),
            Rule::Language => language::check(repo),
            Rule::Private => private::check(repo),
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

impl Level {
    pub fn name(self) -> &'static str {
        match self {
            Level::Warn => "warn",
            Level::Fail => "fail",
        }
    }
}

/// One thing wrong, and how to put it right.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub level: Level,
    pub what: String,
    pub fix: Option<String>,
    /// What `--fix` writes, when putting it right is a file's new contents
    /// and nothing else.
    pub repair: Option<Repair>,
}

impl Finding {
    pub fn fail(what: impl Into<String>) -> Self {
        Self {
            level: Level::Fail,
            what: what.into(),
            fix: None,
            repair: None,
        }
    }

    pub fn warn(what: impl Into<String>) -> Self {
        Self {
            level: Level::Warn,
            what: what.into(),
            fix: None,
            repair: None,
        }
    }

    pub fn fix(mut self, fix: impl Into<String>) -> Self {
        self.fix = Some(fix.into());
        self
    }

    pub fn repair(mut self, repair: Repair) -> Self {
        self.repair = Some(repair);
        self
    }
}

/// A file's new contents, and what writing them does.
///
/// Only an edit that cannot change what the project does qualifies: a line in
/// .gitignore, a pinned package manager, a copy of a master, an icon's images
/// put in order, an audit that only reads. Nothing that runs git, and nothing
/// a person would want to see as a tree before saying yes - that is
/// `lyrn adopt`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repair {
    /// `/`-separated, from the top of the repository.
    pub path: String,
    pub contents: Vec<u8>,
    /// What writing it does, said after the path: "adds the standard's block".
    pub what: String,
}

impl Repair {
    pub fn new(path: impl Into<String>, contents: impl Into<Vec<u8>>, what: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            contents: contents.into(),
            what: what.into(),
        }
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

    pub fn level(&self) -> Option<Level> {
        match self {
            Outcome::Found(findings) => findings.iter().map(|f| f.level).max(),
            _ => None,
        }
    }

    pub fn findings(&self) -> &[Finding] {
        match self {
            Outcome::Found(findings) => findings,
            _ => &[],
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
    /// The text files, read on first use: two rules read every one of them.
    texts: OnceCell<Vec<(String, String)>>,
}

impl Repo {
    pub fn read(path: &Path) -> Repo {
        Repo::read_with(path, &[])
    }

    /// The repository with `written` among its files: what `--fix` has just
    /// written is examined as the commit it is about to become, tracked or
    /// not yet.
    pub fn read_with(path: &Path, written: &[String]) -> Repo {
        let git = Git::open(path);
        let root = git.as_ref().map(|g| g.root().to_path_buf()).unwrap_or_else(|| path.to_path_buf());
        let mut files = match &git {
            Some(git) => git.tracked_files(),
            None => walk(&root),
        };
        for path in written {
            if !files.contains(path) {
                files.push(path.clone());
            }
        }
        files.sort();
        let (workflows, broken_workflows) = workflows::load(&root, &files);
        Repo {
            facts: Facts::read(&root).map_err(|e| e.to_string()),
            git,
            files,
            workflows,
            broken_workflows,
            on_ci: std::env::var_os("CI").is_some_and(|v| !v.is_empty() && v != "false"),
            root,
            texts: OnceCell::new(),
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

    pub fn read_bytes(&self, path: &str) -> Option<Vec<u8>> {
        std::fs::read(self.root.join(path)).ok()
    }

    /// Every file that is text a person wrote, with its contents: not a
    /// lockfile, not an image, not something too large to have been typed.
    pub fn texts(&self) -> &[(String, String)] {
        self.texts.get_or_init(|| {
            self.files
                .iter()
                .filter(|path| is_written_text(path))
                .filter_map(|path| {
                    let file = self.root.join(path);
                    let size = std::fs::metadata(&file).ok()?.len();
                    if size > MAX_TEXT_BYTES {
                        return None;
                    }
                    let bytes = std::fs::read(&file).ok()?;
                    if bytes.contains(&0) {
                        return None;
                    }
                    String::from_utf8(bytes).ok().map(|text| (path.clone(), text))
                })
                .collect()
        })
    }
}

/// Past this a file was generated, not written.
const MAX_TEXT_BYTES: u64 = 1024 * 1024;

/// Whether a path is text someone wrote, judged by its name: a lockfile is
/// written by a package manager, and an image or an archive is not text.
fn is_written_text(path: &str) -> bool {
    const GENERATED: &[&str] = &["Cargo.lock", "pnpm-lock.yaml", "package-lock.json", "yarn.lock", "bun.lockb"];
    const BINARY: &[&str] = &[
        "png",
        "jpg",
        "jpeg",
        "gif",
        "webp",
        "avif",
        "ico",
        "icns",
        "bmp",
        "tif",
        "tiff",
        "heic",
        "jxl",
        "woff",
        "woff2",
        "ttf",
        "otf",
        "eot",
        "pdf",
        "zip",
        "gz",
        "tgz",
        "xz",
        "zst",
        "7z",
        "tar",
        "exe",
        "dll",
        "so",
        "dylib",
        "wasm",
        "mp3",
        "mp4",
        "ogg",
        "wav",
        "flac",
        "webm",
        "mov",
        "bin",
        "db",
        "sqlite",
        "onnx",
        "gguf",
        "safetensors",
    ];
    let name = path.rsplit('/').next().unwrap_or(path);
    if GENERATED.contains(&name) {
        return false;
    }
    let extension = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    !BINARY.contains(&extension.as_str())
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

/// Directories nothing ships from: what is in them is a test's material.
pub const NOT_SHIPPED: &[&str] = &[
    "fixtures",
    "__fixtures__",
    "testdata",
    "test-data",
    "tests",
    "test",
    "__tests__",
    "node_modules",
];

/// Whether a path is a test's material rather than something that ships.
pub fn is_test_material(path: &str) -> bool {
    let mut parts = path.split('/');
    let name = parts.next_back().unwrap_or_default();
    if parts.any(|part| NOT_SHIPPED.contains(&part)) {
        return true;
    }
    let stem = name.split('.').next().unwrap_or(name);
    name.contains(".test.") || name.contains(".spec.") || stem.ends_with("_test") || stem.ends_with("_tests")
}

/// Every rule's outcome for one repository.
fn check_all(repo: &Repo) -> Vec<(Rule, Outcome)> {
    Rule::ALL.iter().map(|&rule| (rule, rule.check(repo))).collect()
}

pub fn run(args: DoctorArgs) -> Result<bool, Box<dyn Error>> {
    if let Some(rule) = args.explain {
        explain(rule);
        return Ok(true);
    }

    let (targets, skipped) = if args.all {
        registry::line()?
    } else {
        let paths = if args.paths.is_empty() {
            vec![PathBuf::from(".")]
        } else {
            args.paths.clone()
        };
        for path in &paths {
            if !path.is_dir() {
                return Err(format!("`{}` is not a directory - `lyrn doctor` examines a repository", path.display()).into());
            }
        }
        (paths.into_iter().map(report::Target::path).collect(), Vec::new())
    };

    let reports = report::examine_all(&targets, args.fix);
    let passed = reports.iter().all(report::Report::passed);
    let standard = stack::Standard::read();
    if args.json {
        println!("{}", report::json(&reports, &skipped, &standard));
    } else if reports.len() == 1 && !args.all {
        print!("{}", report::render_one(&reports[0], &standard));
    } else {
        print!("{}", report::render_table(&reports, &skipped, &standard, args.all));
    }
    Ok(passed)
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

/// The line endings a text file already uses, so an edit keeps them: a file
/// written with `\r\n` and edited with `\n` is a diff of every line.
pub fn newline_of(text: &str) -> &'static str {
    if text.contains("\r\n") { "\r\n" } else { "\n" }
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
    fn a_test_s_material_is_told_from_what_ships() {
        assert!(is_test_material("tests/doctor.rs"));
        assert!(is_test_material("crates/core/tests/corpus/a.md"));
        assert!(is_test_material("web/src/lib/dates.test.ts"));
        assert!(is_test_material("src/parse_test.go"));
        assert!(!is_test_material("src/testing.rs"));
        assert!(!is_test_material("docs/src/content/docs/test-drive.md"));
    }

    #[test]
    fn written_text_is_told_by_its_name() {
        assert!(is_written_text("src/main.rs"));
        assert!(is_written_text("Dockerfile"));
        assert!(!is_written_text("Cargo.lock"));
        assert!(!is_written_text("docs/pnpm-lock.yaml"));
        assert!(!is_written_text("assets/icon.ICO"));
    }
}
