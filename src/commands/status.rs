//! `lyrn status`: where the repository stands, on one screen.
//!
//! The branch against what was pushed, the work tree, the last release and
//! what has been committed since, what GitHub's CI said about the last pushed
//! commit, how old the stack is, and the stage rigger's record says is being
//! built. Each line comes from the place that knows it - git, GitHub, the
//! history, the record - and none of it is kept here.

use std::error::Error;
use std::path::PathBuf;

use chrono::Utc;

use crate::cli::PathArgs;
use crate::commands::check::product::Product;
use crate::commands::doctor::Repo;
use crate::commands::doctor::stack::Stack;
use crate::commands::standing::{Standing, ci_text, stack_text};
use crate::rigger;

pub fn run(args: PathArgs) -> Result<(), Box<dyn Error>> {
    let path = args.path.unwrap_or_else(|| PathBuf::from("."));
    if !path.is_dir() {
        return Err(format!("`{}` is not a directory - `lyrn status` reads a repository", path.display()).into());
    }
    let repo = Repo::read(&path);
    let Some(git) = &repo.git else {
        return Err(format!("`{}` is not in a git repository - `lyrn status` reads one", path.display()).into());
    };
    let product = Product::read(&repo);
    let standing = Standing::read(git);
    let heading = match product.version() {
        Some((_, version)) => format!("{} {version}", product.name),
        None => product.name.clone(),
    };
    println!("lyrn status: {}, {heading}\n", repo.root.display());
    println!("branch    {}", standing.branch_text());
    println!("tree      {}", standing.tree_text());
    match &standing.release {
        Some(release) => println!("released  {}", release.text()),
        None => println!("released  nothing yet - no version tag behind this commit"),
    }
    let pushed = standing
        .upstream
        .as_ref()
        .and_then(|_| git.run(&["rev-parse", "@{u}"]))
        .map(|s| s.trim().to_string());
    match (&product.github, &pushed) {
        (Some(slug), Some(commit)) => {
            let which = if standing.ahead == 0 { "" } else { " (the last pushed commit)" };
            println!("ci        {}{which}", ci_text(slug, commit));
        }
        (None, _) => println!("ci        not on GitHub - no remote names a repository there"),
        (_, None) => println!("ci        nothing pushed to ask about"),
    }
    let now = Utc::now();
    if let Some(updated) = Stack::read(&repo, now).updated {
        println!("stack     {}", stack_text(&updated, now.date_naive()).0);
    }
    println!("stage     {}", stage_line(&repo.root));
    Ok(())
}

/// The stage rigger's record says is being built here.
fn stage_line(root: &std::path::Path) -> String {
    match rigger::project_for(root) {
        Err(reason) => format!("not read - {reason}"),
        Ok(None) => "rigger's record does not know this repository".to_string(),
        Ok(Some(project)) => match rigger::stage(&project) {
            Err(reason) => format!("not read - {reason}"),
            Ok(None) => format!("none: rigger's record has no stage for {project}"),
            Ok(Some(stage)) => {
                let open = match stage.tasks.len() {
                    0 => "every task closed".to_string(),
                    1 => "1 task open".to_string(),
                    n => format!("{n} tasks open"),
                };
                format!("{} · {}, {open} - `rigger context {project}`", stage.version, stage.title)
            }
        },
    }
}
