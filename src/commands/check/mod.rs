//! `lyrn check`: the shopfronts agree with each other and with the product.
//!
//! A product of the line is shown in several places at once - crates.io, npm,
//! GitHub, the documentation site, the installers - and each place renders
//! its own copy of what the product is called, which version it is, what it
//! says about itself and which commands it has. They drift without a sound:
//! nothing fails when `npm/package.json` still says the old number, or the
//! README shows a command that was renamed. Twelve repositories of the line
//! each grew a test file holding some of this; the command holds all of it,
//! the same way everywhere, on every commit rather than on the day of the
//! release.
//!
//! The release itself - bumping the number, writing the changelog, tagging,
//! checking the registries afterwards - is furca's. What `lyrn check` adds is
//! the reading of the standard's layout: which files speak for the product,
//! so that none of them is the one left behind.

pub mod changelog;
mod commands;
mod descriptions;
mod installers;
pub mod product;
mod readme;
mod site;
mod versions;

use std::error::Error;
use std::path::PathBuf;

use serde_json::{Value, json};

use crate::cli::CheckArgs;
use crate::commands::doctor::{Level, Outcome, Repo};
use crate::rigger::Skipped;

use self::product::Product;

/// Where `--explain` sends the reader for the whole page.
const DOCS: &str = "https://lyrn.lacodda.com/reference/check/";

/// A shopfront check, by the name `--explain` takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Shop {
    Versions,
    Changelog,
    Descriptions,
    Readme,
    Commands,
    Site,
    Installers,
}

impl Shop {
    pub const ALL: &'static [Shop] = &[
        Shop::Versions,
        Shop::Changelog,
        Shop::Descriptions,
        Shop::Readme,
        Shop::Commands,
        Shop::Site,
        Shop::Installers,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Shop::Versions => "versions",
            Shop::Changelog => "changelog",
            Shop::Descriptions => "descriptions",
            Shop::Readme => "readme",
            Shop::Commands => "commands",
            Shop::Site => "site",
            Shop::Installers => "installers",
        }
    }

    pub fn summary(self) -> &'static str {
        match self {
            Shop::Versions => "Every manifest of the product says one version, and so do the wrapper's pin and Cargo.lock",
            Shop::Changelog => "The changelog has a section for the version the manifests say",
            Shop::Descriptions => "The product says the same thing about itself on every shopfront",
            Shop::Readme => "One README for every registry, and its links work away from GitHub",
            Shop::Commands => "Every command the README and the docs show exists, and every command has a page",
            Shop::Site => "The documentation site agrees with itself and the manifests about where it lives",
            Shop::Installers => "The installers fetch this repository's releases, and name the crate it publishes",
        }
    }

    pub fn lesson(self) -> &'static str {
        match self {
            Shop::Versions => include_str!("lessons/versions.txt"),
            Shop::Changelog => include_str!("lessons/changelog.txt"),
            Shop::Descriptions => include_str!("lessons/descriptions.txt"),
            Shop::Readme => include_str!("lessons/readme.txt"),
            Shop::Commands => include_str!("lessons/commands.txt"),
            Shop::Site => include_str!("lessons/site.txt"),
            Shop::Installers => include_str!("lessons/installers.txt"),
        }
    }

    fn check(self, repo: &Repo, product: &Product) -> Outcome {
        match self {
            Shop::Versions => versions::check(repo, product),
            Shop::Changelog => changelog::check(repo, product),
            Shop::Descriptions => descriptions::check(repo, product),
            Shop::Readme => readme::check(repo, product),
            Shop::Commands => commands::check(repo, product),
            Shop::Site => site::check(repo, product),
            Shop::Installers => installers::check(repo, product),
        }
    }
}

/// One repository, checked.
struct Report {
    name: String,
    root: PathBuf,
    missing: Option<String>,
    product: Option<String>,
    outcomes: Vec<(Shop, Outcome)>,
}

impl Report {
    fn level(&self) -> Option<Level> {
        self.outcomes.iter().filter_map(|(_, o)| o.level()).max()
    }

    fn passed(&self) -> bool {
        self.missing.is_none() && self.level().is_none_or(|l| l < Level::Fail)
    }

    fn result(&self) -> &'static str {
        if self.missing.is_some() {
            return "missing";
        }
        match self.level() {
            Some(Level::Fail) => "fail",
            Some(Level::Warn) => "warn",
            None => "pass",
        }
    }
}

pub fn run(args: CheckArgs) -> Result<bool, Box<dyn Error>> {
    if let Some(shop) = args.explain {
        println!("{} - {}\n", shop.name(), shop.summary());
        println!("{}", shop.lesson().trim_end());
        println!("\nMore: {DOCS}#{}", shop.name());
        return Ok(true);
    }
    let (targets, skipped): (Vec<(String, PathBuf)>, Vec<Skipped>) = if args.all {
        crate::rigger::line()?
    } else {
        let paths = if args.paths.is_empty() {
            vec![PathBuf::from(".")]
        } else {
            args.paths.clone()
        };
        for path in &paths {
            if !path.is_dir() {
                return Err(format!("`{}` is not a directory - `lyrn check` checks a repository", path.display()).into());
            }
        }
        let named = paths
            .into_iter()
            .map(|path| {
                let name = std::fs::canonicalize(&path)
                    .ok()
                    .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                    .unwrap_or_else(|| path.display().to_string());
                (name, path)
            })
            .collect();
        (named, Vec::new())
    };

    let reports: Vec<Report> = std::thread::scope(|scope| {
        let handles: Vec<_> = targets.iter().map(|(name, path)| scope.spawn(move || examine(name, path))).collect();
        handles.into_iter().map(|h| h.join().expect("checking a repository does not panic")).collect()
    });
    let passed = reports.iter().all(Report::passed);
    if args.json {
        println!("{}", render_json(&reports, &skipped));
    } else if reports.len() == 1 && !args.all {
        print!("{}", render_one(&reports[0]));
    } else {
        print!("{}", render_table(&reports, &skipped, args.all));
    }
    Ok(passed)
}

fn examine(name: &str, path: &std::path::Path) -> Report {
    if !path.is_dir() {
        return Report {
            name: name.to_string(),
            root: path.to_path_buf(),
            missing: Some(format!("{} is not a directory", path.display())),
            product: None,
            outcomes: Vec::new(),
        };
    }
    let repo = Repo::read(path);
    let product = Product::read(&repo);
    let outcomes = check_all(&repo, &product);
    let version = product.version().map(|(_, v)| format!(" {v}")).unwrap_or_default();
    Report {
        name: name.to_string(),
        root: repo.root.clone(),
        missing: None,
        product: Some(format!("{}{version}", product.name)),
        outcomes,
    }
}

/// Every check's outcome for one repository.
pub fn check_all(repo: &Repo, product: &Product) -> Vec<(Shop, Outcome)> {
    Shop::ALL.iter().map(|&shop| (shop, shop.check(repo, product))).collect()
}

fn render_one(report: &Report) -> String {
    let mut out = match &report.product {
        Some(product) => format!("lyrn check: {}, {product}\n\n", report.root.display()),
        None => format!("lyrn check: {}\n\n", report.root.display()),
    };
    if let Some(reason) = &report.missing {
        out.push_str(&format!("✗ {reason}\n"));
        return out;
    }
    const COLUMN: usize = 14;
    let (mut failed, mut warned, mut passed, mut skipped) = (0, 0, 0, 0);
    for (shop, outcome) in &report.outcomes {
        let name = shop.name();
        match outcome {
            Outcome::Pass(text) => {
                passed += 1;
                out.push_str(&format!("✓ {name:<COLUMN$}{text}\n"));
            }
            Outcome::Skip(text) => {
                skipped += 1;
                out.push_str(&format!("- {name:<COLUMN$}{text}\n"));
            }
            Outcome::Found(findings) => {
                let fail = outcome.level() == Some(Level::Fail);
                if fail {
                    failed += 1
                } else {
                    warned += 1
                }
                out.push_str(&format!("{} {name}\n", if fail { '✗' } else { '!' }));
                for finding in findings {
                    let sign = if finding.level == Level::Fail { '✗' } else { '!' };
                    out.push_str(&format!("    {sign} {}\n", finding.what));
                    if let Some(fix) = &finding.fix {
                        for (i, line) in fix.lines().enumerate() {
                            out.push_str(&format!("      {} {line}\n", if i == 0 { '→' } else { ' ' }));
                        }
                    }
                }
                out.push_str(&format!("    why: lyrn check --explain {name}\n"));
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
    out.push_str(&format!("\n{}\n", parts.join(", ")));
    out
}

fn render_table(reports: &[Report], skipped: &[Skipped], all: bool) -> String {
    let from = if all { " from rigger's record" } else { "" };
    let count = if reports.len() == 1 {
        "1 repository".to_string()
    } else {
        format!("{} repositories", reports.len())
    };
    let mut out = format!("lyrn check: {count}{from}\n\n");
    let width = reports.iter().map(|r| r.name.chars().count()).max().unwrap_or(4) + 2;
    out.push_str(&format!("{:width$}{}\n", "", Shop::ALL.iter().map(|s| s.name()).collect::<Vec<_>>().join(" ")));
    for report in reports {
        out.push_str(&format!("{:width$}", report.name));
        if let Some(reason) = &report.missing {
            out.push_str(&format!("✗ {reason}\n"));
            continue;
        }
        let cells: Vec<String> = Shop::ALL
            .iter()
            .map(|shop| {
                let mark = match report.outcomes.iter().find(|(s, _)| s == shop).map(|(_, o)| o) {
                    Some(Outcome::Pass(_)) => '✓',
                    Some(o @ Outcome::Found(_)) if o.level() == Some(Level::Fail) => '✗',
                    Some(Outcome::Found(_)) => '!',
                    _ => '-',
                };
                format!("{mark:<w$}", w = shop.name().len())
            })
            .collect();
        out.push_str(cells.join(" ").trim_end());
        out.push('\n');
    }
    if !skipped.is_empty() {
        out.push('\n');
        for skip in skipped {
            out.push_str(&format!("Not checked: {} - {}\n", skip.name, skip.reason));
        }
    }
    let failing = reports.iter().filter(|r| matches!(r.result(), "fail" | "missing")).count();
    let passing = reports.len() - failing;
    out.push_str(&format!(
        "\n{failing} {}, {passing} {}\n`lyrn check <path>` says what is off in one of them and how to put it right\n",
        if failing == 1 { "fails" } else { "fail" },
        if passing == 1 { "passes" } else { "pass" }
    ));
    out
}

fn render_json(reports: &[Report], skipped: &[Skipped]) -> String {
    let repositories: Vec<Value> = reports
        .iter()
        .map(|report| {
            let checks: Vec<Value> = report
                .outcomes
                .iter()
                .map(|(shop, outcome)| {
                    let (result, text) = match outcome {
                        Outcome::Pass(text) => ("pass", Some(text.as_str())),
                        Outcome::Skip(text) => ("skip", Some(text.as_str())),
                        Outcome::Found(_) => (outcome.level().map_or("warn", Level::name), None),
                    };
                    let findings: Vec<Value> = outcome
                        .findings()
                        .iter()
                        .map(|f| json!({ "level": f.level.name(), "what": f.what, "fix": f.fix }))
                        .collect();
                    json!({ "check": shop.name(), "result": result, "text": text, "findings": findings })
                })
                .collect();
            json!({
                "name": report.name,
                "path": report.root.display().to_string(),
                "product": report.product,
                "result": report.result(),
                "reason": report.missing,
                "checks": checks,
            })
        })
        .collect();
    let document = json!({
        "lyrn": env!("CARGO_PKG_VERSION"),
        "repositories": repositories,
        "skipped": skipped.iter().map(|s| json!({ "name": s.name, "reason": s.reason })).collect::<Vec<_>>(),
    });
    serde_json::to_string_pretty(&document).expect("a document of strings serialises")
}

/// The same text, as a reader would compare it: spaces collapsed, curly
/// quotes read as straight ones, and no full stop at the end - a caption and
/// a sentence say the same thing.
pub fn same_words(a: &str, b: &str) -> bool {
    normalize(a) == normalize(b)
}

fn normalize(text: &str) -> String {
    let straight: String = text
        .chars()
        .map(|c| match c {
            '\u{2018}' | '\u{2019}' => '\'',
            '\u{201C}' | '\u{201D}' => '"',
            c => c,
        })
        .collect();
    straight.split_whitespace().collect::<Vec<_>>().join(" ").trim_end_matches('.').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_check_is_explained_under_the_name_it_is_reported_by() {
        use clap::ValueEnum;
        for shop in Shop::ALL {
            let value = shop.to_possible_value().expect("every check can be named");
            assert_eq!(value.get_name(), shop.name());
            assert!(!shop.lesson().trim().is_empty(), "{} has no lesson", shop.name());
        }
        assert_eq!(Shop::ALL.len(), Shop::value_variants().len(), "a check is missing from Shop::ALL");
    }

    #[test]
    fn a_caption_and_a_sentence_say_the_same() {
        assert!(same_words("A fast git client.", "A fast  git client"));
        assert!(same_words("employees\u{2019} agents", "employees' agents"));
        assert!(!same_words("A fast git client", "A quick git client"));
    }
}
