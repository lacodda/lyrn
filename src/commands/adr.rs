//! `lyrn adr new`: a decision written down, under the next number of the log.
//!
//! A decision record is cheap to write and easy to put off, and what makes it
//! a chore is the part around the writing: finding the next number, keeping the
//! file's name in the log's shape, listing it in the index. The number is read
//! from the directory at the moment of writing, so two decisions on one
//! afternoon cannot both be 0012.

use std::error::Error;
use std::path::PathBuf;

use crate::cli::AdrCommand;
use crate::naming;
use crate::repo::{self, DecisionLog};

pub fn run(command: AdrCommand) -> Result<(), Box<dyn Error>> {
    match command {
        AdrCommand::New { title, path } => new(&title, path),
    }
}

fn new(title: &str, path: Option<PathBuf>) -> Result<(), Box<dyn Error>> {
    let root = path.unwrap_or_else(|| PathBuf::from("."));
    if !root.is_dir() {
        return Err(format!("`{}` is not a directory - `lyrn adr new` writes into a repository's log", root.display()).into());
    }
    let title = naming::one_line(title);
    let slug = naming::slug(&title);
    if slug.is_empty() {
        return Err(format!("`{title}` leaves nothing for a file's name: write the title in English").into());
    }
    let log = repo::decision_log(&root).ok_or("this repository keeps no decisions in docs/adr/ - `lyrn adopt` starts the log")?;

    let number = format!("{:04}", log.next_number());
    let name = format!("{number}-{slug}.md");
    let file = root.join(&log.dir).join(&name);
    let date = chrono::Utc::now().format("%Y-%m-%d");
    std::fs::write(&file, record(&number, &title, &date.to_string())).map_err(|e| format!("{}: {e}", file.display()))?;
    println!("Wrote {}/{name}.", log.dir);

    match list_it(&root, &log, &number, &name, &title)? {
        true => println!("Listed it in {}/README.md.", log.dir),
        false => println!("Its index has no table of decisions to add it to: {}/README.md is left as it was.", log.dir),
    }
    Ok(())
}

/// The record, with what each part is for where its text goes.
fn record(number: &str, title: &str, date: &str) -> String {
    format!(
        "# ADR {number}: {title}\n\n\
         - Status: accepted\n\
         - Date: {date}\n\n\
         ## Context\n\n\
         <!-- What forces a decision now, and what makes it hard. -->\n\n\
         ## Decision\n\n\
         <!-- What was decided, in the present tense. -->\n\n\
         ## Consequences\n\n\
         <!-- What becomes easier, what becomes harder, and what is given up. -->\n"
    )
}

/// The log's index with the decision's row after the last one; whether there
/// was a table to add it to. An index without one is the repository's own idea
/// of an index, and is not rewritten into the standard's.
fn list_it(root: &std::path::Path, log: &DecisionLog, number: &str, name: &str, title: &str) -> Result<bool, Box<dyn Error>> {
    let file = root.join(&log.dir).join("README.md");
    let Ok(text) = std::fs::read_to_string(&file) else { return Ok(false) };
    let Some(listed) = with_a_row(&text, number, name, title) else {
        return Ok(false);
    };
    std::fs::write(&file, listed).map_err(|e| format!("{}: {e}", file.display()))?;
    Ok(true)
}

/// The index with a row after its last decision, in the line endings it has.
fn with_a_row(text: &str, number: &str, name: &str, title: &str) -> Option<String> {
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let row = format!("| [{number}]({name}) | {} | Accepted |", title.replace('|', "\\|"));
    let mut at = None;
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        offset += line.len();
        if line.trim_start().starts_with("| [") {
            at = Some(offset);
        }
    }
    let at = at?;
    let (before, after) = text.split_at(at);
    let glue = if before.ends_with('\n') { "" } else { newline };
    Some(format!("{before}{glue}{row}{newline}{after}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_row_goes_after_the_last_decision() {
        let index = "# Decisions\n\n| # | Decision | Status |\n| --- | --- | --- |\n| [0001](0001-a.md) | A | Accepted |\n\nMore words.\n";
        assert_eq!(
            with_a_row(index, "0002", "0002-b.md", "B").unwrap(),
            "# Decisions\n\n| # | Decision | Status |\n| --- | --- | --- |\n| [0001](0001-a.md) | A | Accepted |\n| [0002](0002-b.md) | B | Accepted |\n\nMore words.\n"
        );
        let crlf = "| [0001](0001-a.md) | A | Accepted |\r\n";
        assert_eq!(
            with_a_row(crlf, "0002", "0002-b.md", "B").unwrap(),
            "| [0001](0001-a.md) | A | Accepted |\r\n| [0002](0002-b.md) | B | Accepted |\r\n"
        );
        assert_eq!(with_a_row("# Our decisions, in prose\n", "0002", "0002-b.md", "B"), None);
    }
}
