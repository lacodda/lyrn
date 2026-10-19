//! What the doctor says: one repository at length, many as a table, or
//! either as one JSON document - and, with `--fix`, the repairs written
//! before the repository is examined again.

use std::path::{Path, PathBuf};

use chrono::Utc;
use serde_json::{Value, json};

use super::stack::{Stack, Standard};
use super::{Level, Outcome, Repair, Repo, Rule, check_all};
use crate::rigger::Skipped;

/// A repository to examine, by the name a table shows it under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub name: String,
    pub path: PathBuf,
}

impl Target {
    /// A path given on the command line, named after its directory.
    pub fn path(path: PathBuf) -> Target {
        let name = std::fs::canonicalize(&path)
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .unwrap_or_else(|| path.display().to_string());
        Target { name, path }
    }

    pub fn named(name: &str, path: PathBuf) -> Target {
        Target { name: name.to_string(), path }
    }
}

/// A repair `--fix` wrote, or could not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Written {
    pub path: String,
    pub what: String,
    pub error: Option<String>,
}

/// One repository, examined.
#[derive(Debug)]
pub struct Report {
    pub name: String,
    pub root: PathBuf,
    /// Why the repository could not be examined at all.
    pub missing: Option<String>,
    pub form: Option<String>,
    pub standard: Option<String>,
    pub outcomes: Vec<(Rule, Outcome)>,
    pub stack: Stack,
    pub written: Vec<Written>,
}

impl Report {
    pub fn passed(&self) -> bool {
        self.missing.is_none() && self.level().is_none_or(|l| l < Level::Fail)
    }

    /// The worst level among the findings.
    fn level(&self) -> Option<Level> {
        self.outcomes.iter().filter_map(|(_, o)| o.level()).max()
    }

    /// The findings a `--fix` would put right.
    fn repairs(&self) -> impl Iterator<Item = &Repair> {
        self.outcomes.iter().flat_map(|(_, o)| o.findings()).filter_map(|f| f.repair.as_ref())
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

/// Every target, examined side by side: the repositories have nothing to
/// share, and the slowest is a history read end to end.
pub fn examine_all(targets: &[Target], fix: bool) -> Vec<Report> {
    std::thread::scope(|scope| {
        let handles: Vec<_> = targets.iter().map(|t| scope.spawn(move || examine(t, fix))).collect();
        handles.into_iter().map(|h| h.join().expect("examining a repository does not panic")).collect()
    })
}

fn examine(target: &Target, fix: bool) -> Report {
    if !target.path.is_dir() {
        return Report {
            name: target.name.clone(),
            root: target.path.clone(),
            missing: Some(format!("{} is not a directory", target.path.display())),
            form: None,
            standard: None,
            outcomes: Vec::new(),
            stack: Stack::default(),
            written: Vec::new(),
        };
    }
    let mut repo = Repo::read(&target.path);
    let mut outcomes = check_all(&repo);
    let mut written = Vec::new();
    if fix {
        let repairs: Vec<Repair> = outcomes.iter().flat_map(|(_, o)| o.findings()).filter_map(|f| f.repair.clone()).collect();
        for repair in repairs {
            written.push(write(&repo.root, &repair));
        }
        if !written.is_empty() {
            let paths: Vec<String> = written.iter().filter(|w| w.error.is_none()).map(|w| w.path.clone()).collect();
            repo = Repo::read_with(&target.path, &paths);
            outcomes = check_all(&repo);
        }
    }
    let (form, standard) = match &repo.facts {
        Ok(facts) => (facts.form(&repo.root).ok().map(|(form, _)| form.to_string()), facts.standard.clone()),
        Err(_) => (None, None),
    };
    Report {
        name: target.name.clone(),
        stack: Stack::read(&repo, Utc::now()),
        root: repo.root,
        missing: None,
        form,
        standard,
        outcomes,
        written,
    }
}

fn write(root: &Path, repair: &Repair) -> Written {
    let file = root.join(&repair.path);
    let result = file
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| std::fs::write(&file, &repair.contents));
    Written {
        path: repair.path.clone(),
        what: repair.what.clone(),
        error: result.err().map(|e| e.to_string()),
    }
}

/// One repository, rule by rule.
pub fn render_one(report: &Report, standard: &Standard) -> String {
    let mut out = format!("{}\n\n", heading(report));
    if let Some(reason) = &report.missing {
        out.push_str(&format!("✗ {reason}\n"));
        return out;
    }
    if !report.written.is_empty() {
        out.push_str("Fixed:\n");
        for written in &report.written {
            out.push_str(&written_line(written, ""));
        }
        out.push('\n');
    }
    for (rule, outcome) in &report.outcomes {
        out.push_str(&render(*rule, outcome));
    }
    let mut counts = Counts::default();
    for (_, outcome) in &report.outcomes {
        counts.add(outcome);
    }
    out.push_str(&format!("\n{}{}\n", counts.summary(), fixable(report.repairs().count(), "")));
    if let Some(line) = stack_line(&report.stack, standard) {
        out.push_str(&format!("stack: {line}\n"));
    }
    out
}

/// A repository's stack in one line, each cell behind the standard saying
/// what the standard is on; `None` when there is nothing to say.
pub fn stack_line(stack: &Stack, standard: &Standard) -> Option<String> {
    let cells = stack_cells(stack, standard);
    let line: Vec<String> = cells
        .iter()
        .filter(|c| c.text != "-")
        .map(|c| {
            let text = format!("{} {}", c.column, c.text);
            // The cells that say it themselves - "2 behind", "skipped it" -
            // need no word more.
            let on = match c.column {
                "edition" => Some(&standard.edition),
                "pnpm" => Some(&standard.pnpm),
                _ => None,
            };
            match (c.behind, on, c.column) {
                (true, Some(on), _) => format!("{text} (the standard is on {on})"),
                (true, None, "node") => format!("{text} (the standard is on {})", standard.node),
                _ => text,
            }
        })
        .collect();
    (!line.is_empty()).then(|| line.join(" · "))
}

/// The first line of a report: which repository, and what lyrn takes it for.
fn heading(report: &Report) -> String {
    let place = report.root.display().to_string();
    match (&report.form, &report.standard) {
        (Some(form), Some(standard)) => format!("lyrn doctor: {place}, a `{form}` project on standard {standard}"),
        (Some(form), None) => format!("lyrn doctor: {place}, a `{form}` project"),
        (None, _) => format!("lyrn doctor: {place}"),
    }
}

fn written_line(written: &Written, prefix: &str) -> String {
    match &written.error {
        None => format!("  ✓ {prefix}{}: {}\n", written.path, written.what),
        Some(error) => format!("  ✗ {prefix}{}: could not write it - {error}\n", written.path),
    }
}

/// What `--fix` would do, said after the summary.
fn fixable(count: usize, all: &str) -> String {
    match count {
        0 => String::new(),
        1 => format!("; `lyrn doctor{all} --fix` puts 1 finding right"),
        n => format!("; `lyrn doctor{all} --fix` puts {n} findings right"),
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

/// A rule's outcome as one mark: the table's cell, the report's first column.
fn mark(outcome: &Outcome) -> char {
    match (outcome, outcome.level()) {
        (Outcome::Pass(_), _) => '✓',
        (Outcome::Skip(_), _) => '-',
        (_, Some(Level::Fail)) => '✗',
        _ => '!',
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
        match mark(outcome) {
            '✓' => self.passed += 1,
            '-' => self.skipped += 1,
            '✗' => self.failed += 1,
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

/// One cell of the stack, and whether it is behind the standard.
struct Cell {
    column: &'static str,
    text: String,
    behind: bool,
}

fn stack_cells(stack: &Stack, standard: &Standard) -> Vec<Cell> {
    let behind = stack.behind(standard);
    let cell = |column: &'static str, text: String| Cell {
        column,
        behind: behind.contains(&column),
        text,
    };
    let msrv = match (&stack.msrv, &stack.toolchain) {
        (Some(msrv), Some(pin)) => format!("{msrv}, pins {pin}"),
        (Some(msrv), None) => msrv.clone(),
        (None, Some(pin)) => format!("pins {pin}"),
        (None, None) => "-".to_string(),
    };
    let actions = match &stack.actions_behind {
        None => "-".to_string(),
        Some(list) if list.is_empty() => "on the standard".to_string(),
        Some(list) => format!("{} behind", list.len()),
    };
    let updated = match &stack.updated {
        None => "-".to_string(),
        Some(updated) => {
            let mut text = match &updated.last {
                None => "never".to_string(),
                Some((_, 0)) => "today".to_string(),
                Some((_, 1)) => "yesterday".to_string(),
                Some((_, days)) => format!("{days} days ago"),
            };
            if updated.last.as_ref().is_some_and(|(_, days)| *days >= 7) {
                text.push_str(", due");
            }
            if let Some(tag) = &updated.skipped_by {
                text.push_str(&format!(", {tag} skipped it"));
            }
            text
        }
    };
    vec![
        cell("edition", stack.edition.clone().unwrap_or_else(|| "-".to_string())),
        cell("msrv", msrv),
        cell("node", stack.node.map_or_else(|| "-".to_string(), |n| n.to_string())),
        cell("pnpm", stack.pnpm.clone().unwrap_or_else(|| "-".to_string())),
        cell("actions", actions),
        cell("updated", updated),
    ]
}

/// Many repositories: a mark per rule, then the stack side by side.
pub fn render_table(reports: &[Report], skipped: &[Skipped], standard: &Standard, all: bool) -> String {
    let from = if all { " from rigger's record" } else { "" };
    let mut out = format!(
        "lyrn doctor: {}{from}\n\n",
        if reports.len() == 1 {
            "1 repository".to_string()
        } else {
            format!("{} repositories", reports.len())
        }
    );
    let name_width = reports.iter().map(|r| r.name.chars().count()).chain(["standard".len()]).max().unwrap_or(8) + 2;

    out.push_str(&format!("{:name_width$}", ""));
    out.push_str(&Rule::ALL.iter().map(|r| r.name()).collect::<Vec<_>>().join(" "));
    out.push('\n');
    for report in reports {
        out.push_str(&format!("{:name_width$}", report.name));
        if let Some(reason) = &report.missing {
            out.push_str(&format!("✗ {reason}\n"));
            continue;
        }
        let cells: Vec<String> = Rule::ALL
            .iter()
            .map(|rule| {
                let outcome = report.outcomes.iter().find(|(r, _)| r == rule).map(|(_, o)| o);
                let mark = outcome.map_or('-', mark);
                format!("{mark:<width$}", width = rule.name().len())
            })
            .collect();
        out.push_str(cells.join(" ").trim_end());
        out.push('\n');
    }
    if !skipped.is_empty() {
        out.push('\n');
        for skip in skipped {
            out.push_str(&format!("Not examined: {} - {}\n", skip.name, skip.reason));
        }
    }

    let written: Vec<String> = reports
        .iter()
        .flat_map(|r| r.written.iter().map(move |w| written_line(w, &format!("{}/", r.name))))
        .collect();
    if !written.is_empty() {
        out.push_str("\nFixed:\n");
        out.push_str(&written.concat());
    }

    let failing = reports.iter().filter(|r| r.result() == "fail" || r.result() == "missing").count();
    let warned = reports.iter().filter(|r| r.result() == "warn").count();
    let passing = reports.iter().filter(|r| r.result() == "pass").count();
    let mut parts = Vec::new();
    if failing > 0 {
        parts.push(format!("{failing} {}", if failing == 1 { "fails" } else { "fail" }));
    }
    if warned > 0 {
        parts.push(format!("{warned} {} something to look at", if warned == 1 { "has" } else { "have" }));
    }
    parts.push(format!("{passing} {}", if passing == 1 { "passes" } else { "pass" }));
    let repairs: usize = reports.iter().map(|r| r.repairs().count()).sum();
    out.push_str(&format!(
        "\n{}{}\n`lyrn doctor <path>` says what is off in one of them, how to put it right and why\n",
        parts.join(", "),
        fixable(repairs, if all { " --all" } else { "" })
    ));

    // The stack, side by side, under the standard's own row.
    let mut rows: Vec<(String, Vec<Cell>)> = vec![(
        "standard".to_string(),
        vec![
            Cell {
                column: "edition",
                text: standard.edition.clone(),
                behind: false,
            },
            Cell {
                column: "msrv",
                text: String::new(),
                behind: false,
            },
            Cell {
                column: "node",
                text: standard.node.to_string(),
                behind: false,
            },
            Cell {
                column: "pnpm",
                text: standard.pnpm.clone(),
                behind: false,
            },
            Cell {
                column: "actions",
                text: String::new(),
                behind: false,
            },
            Cell {
                column: "updated",
                text: "weekly".to_string(),
                behind: false,
            },
        ],
    )];
    for report in reports.iter().filter(|r| r.missing.is_none()) {
        rows.push((report.name.clone(), stack_cells(&report.stack, standard)));
    }
    let columns = ["edition", "msrv", "node", "pnpm", "actions", "updated"];
    let shown = |cell: &Cell| if cell.behind { format!("{}*", cell.text) } else { cell.text.clone() };
    let widths: Vec<usize> = columns
        .iter()
        .enumerate()
        .map(|(i, column)| {
            rows.iter()
                .map(|(_, cells)| shown(&cells[i]).chars().count())
                .chain([column.len()])
                .max()
                .unwrap_or(0)
                + 2
        })
        .collect();
    out.push_str(&format!("\n{:name_width$}", "stack"));
    for (column, width) in columns.iter().zip(&widths) {
        out.push_str(&format!("{column:width$}"));
    }
    out = out.trim_end_matches(' ').to_string();
    out.push('\n');
    for (name, cells) in &rows {
        let mut line = format!("{name:name_width$}");
        for (cell, width) in cells.iter().zip(&widths) {
            line.push_str(&format!("{:width$}", shown(cell)));
        }
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out.push_str("* behind the standard\n");
    out
}

/// One document for one repository or many: the same shape either way, so
/// whatever reads it never has two to understand.
pub fn json(reports: &[Report], skipped: &[Skipped], standard: &Standard) -> String {
    let repositories: Vec<Value> = reports
        .iter()
        .map(|report| {
            let rules: Vec<Value> = report
                .outcomes
                .iter()
                .map(|(rule, outcome)| {
                    let (result, text) = match outcome {
                        Outcome::Pass(text) => ("pass", Some(text.as_str())),
                        Outcome::Skip(text) => ("skip", Some(text.as_str())),
                        Outcome::Found(_) => (outcome.level().map_or("warn", Level::name), None),
                    };
                    let findings: Vec<Value> = outcome
                        .findings()
                        .iter()
                        .map(|f| {
                            json!({
                                "level": f.level.name(),
                                "what": f.what,
                                "fix": f.fix,
                                "repair": f.repair.as_ref().map(|r| json!({ "path": r.path, "what": r.what })),
                            })
                        })
                        .collect();
                    json!({ "rule": rule.name(), "result": result, "text": text, "findings": findings })
                })
                .collect();
            let stack = &report.stack;
            json!({
                "name": report.name,
                "path": report.root.display().to_string(),
                "result": report.result(),
                "reason": report.missing,
                "form": report.form,
                "standard": report.standard,
                "rules": rules,
                "fixed": report.written.iter().map(|w| json!({ "path": w.path, "what": w.what, "error": w.error })).collect::<Vec<_>>(),
                "stack": {
                    "edition": stack.edition,
                    "msrv": stack.msrv,
                    "toolchain": stack.toolchain,
                    "node": stack.node,
                    "pnpm": stack.pnpm,
                    "actions_behind": stack.actions_behind,
                    "updated": stack.updated.as_ref().map(|u| json!({
                        "last": u.last.as_ref().map(|(day, _)| day),
                        "days": u.last.as_ref().map(|(_, days)| days),
                        "skipped_by": u.skipped_by,
                    })),
                    "behind": stack.behind(standard),
                },
            })
        })
        .collect();
    let document = json!({
        "lyrn": env!("CARGO_PKG_VERSION"),
        "standard": {
            "edition": standard.edition,
            "node": standard.node,
            "pnpm": standard.pnpm,
            "actions": standard.actions,
        },
        "repositories": repositories,
        "skipped": skipped.iter().map(|s| json!({ "name": s.name, "reason": s.reason })).collect::<Vec<_>>(),
    });
    serde_json::to_string_pretty(&document).expect("a document of strings and numbers serialises")
}

#[cfg(test)]
mod tests {
    use super::super::Finding;
    use super::*;

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
