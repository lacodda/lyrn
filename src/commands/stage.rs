//! `lyrn stage`: the ritual that starts a stage, as one command.
//!
//! A stage is one version, built from the record's plan to a tag. Before its
//! first commit the line asks the same few things every time: what the stage
//! is, whether the last release really went out, whether the tree is clean
//! and level with what was pushed, whether the stack is due its weekly
//! update - which is then the stage's first commit - and what the doctor and
//! the shopfront check say. The command asks them, says what stands in the
//! way, and leaves one line in rigger's record, so the stage's start is in
//! the diary next to the work it began and not only in git.

use std::error::Error;
use std::path::PathBuf;

use chrono::Utc;

use crate::cli::PathArgs;
use crate::commands::check::{self, product::Product};
use crate::commands::doctor::stack::Stack;
use crate::commands::doctor::{self, Level, Outcome, Repo};
use crate::commands::standing::{Standing, ci_text, stack_text};
use crate::rigger;

pub fn run(args: PathArgs) -> Result<(), Box<dyn Error>> {
    let path = args.path.unwrap_or_else(|| PathBuf::from("."));
    if !path.is_dir() {
        return Err(format!("`{}` is not a directory - `lyrn stage` starts a stage in a repository", path.display()).into());
    }
    let repo = Repo::read(&path);
    let Some(git) = &repo.git else {
        return Err(format!("`{}` is not in a git repository - a stage ends in a tag, and a tag needs one", path.display()).into());
    };
    let product = Product::read(&repo);
    let standing = Standing::read(git);
    println!("lyrn stage: {}, {}\n", repo.root.display(), product.name);

    let mut in_the_way: Vec<String> = Vec::new();
    let mut facts: Vec<String> = Vec::new();

    let project = rigger::project_for(&repo.root);
    let stage = match &project {
        Ok(Some(name)) => rigger::stage(name),
        Ok(None) => Err("rigger's record does not know this repository".to_string()),
        Err(reason) => Err(reason.clone()),
    };
    match &stage {
        Ok(Some(stage)) => {
            println!("stage     {} · {}, from rigger's record", stage.version, stage.title);
            for (id, task) in &stage.tasks {
                println!("            [{id}] {}", first_line(task));
            }
            facts.push(format!("Stage {} began", stage.version));
            if let Some(release) = &standing.release
                && crate::registries::version_cmp(&stage.version, &release.tag).is_le()
            {
                in_the_way.push(format!(
                    "the stage is {}, and {} is already tagged: the record's plan is behind git",
                    stage.version, release.tag
                ));
            }
        }
        Ok(None) => {
            println!("stage     none - rigger's record has no stage for this project; plan one first");
            in_the_way.push("rigger's record names no stage to start".to_string());
        }
        Err(reason) => println!("stage     not read - {reason}"),
    }
    if facts.is_empty() {
        facts.push("A stage began".to_string());
    }
    if let Some(head) = &standing.head {
        facts[0].push_str(&format!(" at {}", &head[..head.len().min(7)]));
    }

    match &standing.release {
        Some(release) => {
            let ci = product.github.as_ref().map(|slug| ci_text(slug, &release.commit));
            let ci_part = ci.as_ref().map_or(String::new(), |c| format!("; CI {c}"));
            println!("release   {}{ci_part}", release.text());
            if ci.as_deref().is_some_and(|c| c.starts_with("red") || c.starts_with("still running")) {
                in_the_way.push(format!("the CI of {} is not green: the last release is not finished", release.tag));
            }
            facts.push(format!("{} out on {}", release.tag, release.day));
        }
        None => println!("release   nothing released yet"),
    }

    let tree = format!("{}, {}", standing.tree_text(), standing.branch_text());
    println!("tree      {tree}");
    if !standing.clean() {
        in_the_way.push("the work tree is not clean: commit the changes, or put them away".to_string());
    }
    if standing.ahead > 0 {
        in_the_way.push("commits are waiting to be pushed: the last stage is not finished until they are".to_string());
    }
    if standing.behind > 0 {
        in_the_way.push("the upstream has commits this branch does not: pull them first".to_string());
    }
    facts.push(format!("tree {}", standing.tree_text()));

    let now = Utc::now();
    if let Some(updated) = Stack::read(&repo, now).updated {
        let (text, due) = stack_text(&updated, now.date_naive());
        if due {
            println!("stack     {text}");
            println!("          the stage starts with one `chore(deps)` commit - `lyrn deps` lists what is behind");
            in_the_way.push("the stack is due: update it first, in one `chore(deps)` commit".to_string());
            facts.push("stack due".to_string());
        } else {
            println!("stack     {text}, so the stage needs no update");
            facts.push(format!("stack {}", text.split(", on ").next().unwrap_or(&text)));
        }
    }

    let doctor_outcomes = doctor::check_all(&repo);
    let doctor_line = summary(doctor_outcomes.iter().map(|(rule, outcome)| (rule.name(), outcome)));
    println!("doctor    {doctor_line}");
    let check_outcomes = check::check_all(&repo, &product);
    let check_line = summary(check_outcomes.iter().map(|(shop, outcome)| (shop.name(), outcome)));
    println!("check     {check_line}");
    facts.push(format!("doctor: {doctor_line}"));
    facts.push(format!("check: {check_line}"));
    if check_outcomes.iter().any(|(_, o)| o.level() == Some(Level::Fail)) {
        in_the_way.push("a shopfront disagrees: `lyrn check` says which".to_string());
    }

    println!();
    if in_the_way.is_empty() {
        println!("Nothing stands in the way.");
    } else {
        println!("Before the stage's first commit:");
        for item in &in_the_way {
            println!("  - {item}");
        }
    }

    // Semicolons between the facts: each of them may hold commas.
    let line = format!("{}.", facts.join("; "));
    match project {
        Ok(Some(name)) => match rigger::note_state(&name, &line) {
            Ok(()) => println!("\nIn rigger's record for {name}: {line}"),
            Err(reason) => println!("\nNot recorded in rigger - {reason}"),
        },
        Ok(None) => println!("\nNot recorded: rigger's record does not know this repository"),
        Err(reason) => println!("\nNot recorded - {reason}"),
    }
    Ok(())
}

/// What a list of outcomes comes to: "1 failed (audit), 12 passed".
fn summary<'a>(outcomes: impl Iterator<Item = (&'a str, &'a Outcome)>) -> String {
    let (mut failed, mut warned, mut passed, mut skipped) = (Vec::new(), Vec::new(), 0, 0);
    for (name, outcome) in outcomes {
        match (outcome, outcome.level()) {
            (Outcome::Pass(_), _) => passed += 1,
            (Outcome::Skip(_), _) => skipped += 1,
            (_, Some(Level::Fail)) => failed.push(name),
            _ => warned.push(name),
        }
    }
    let mut parts = Vec::new();
    if !failed.is_empty() {
        parts.push(format!("{} failed ({})", failed.len(), failed.join(", ")));
    }
    if !warned.is_empty() {
        parts.push(format!("{} to look at ({})", warned.len(), warned.join(", ")));
    }
    parts.push(format!("{passed} passed"));
    if skipped > 0 {
        parts.push(format!("{skipped} not applicable"));
    }
    parts.join(", ")
}

/// A task's first line, without the Markdown emphasis a plan wraps it in.
fn first_line(task: &str) -> String {
    task.lines().next().unwrap_or_default().replace("**", "")
}
