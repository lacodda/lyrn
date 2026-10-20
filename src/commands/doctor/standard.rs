//! `standard` and `mark`: what `lyrn.toml` and the files of the standard say.
//!
//! The files are the ones `lyrn adopt` would add - the same template, the same
//! list, the same rule for a file kept under another name - so the doctor's
//! finding and adopt's tree can never disagree about what is missing.

use std::path::Path;

use super::{Finding, Outcome, Repo, plural, some_of};
use crate::repo::already_there;
use crate::template::{self, Kind};
use crate::templates;

pub fn check(repo: &Repo) -> Outcome {
    // A place `lyrn upgrade` left for a person, still waiting: the file is
    // neither the repository's version nor the standard's. Looked for first,
    // since a lyrn.toml that holds one does not parse.
    let unresolved: Vec<String> = templates::STANDARD
        .iter()
        .filter(|path| repo.read_text(path).is_some_and(|text| text.lines().any(|l| l == "<<<<<<< yours")))
        .map(|path| path.to_string())
        .collect();
    let mut findings = Vec::new();
    if !unresolved.is_empty() {
        findings.push(
            Finding::fail(format!(
                "{} {} a place `lyrn upgrade` left for a hand: {}",
                plural(unresolved.len(), "file", "files"),
                if unresolved.len() == 1 { "keeps" } else { "keep" },
                some_of(&unresolved, 5)
            ))
            .fix("keep what both sides meant between `<<<<<<< yours` and `>>>>>>> standard ...`, drop the markers, commit"),
        );
    }

    let facts = match &repo.facts {
        Ok(facts) => facts,
        Err(reason) => {
            findings.push(Finding::fail(reason.clone()).fix("fix the manifest; nothing can read it as it is"));
            return Outcome::Found(findings);
        }
    };
    let form = match facts.form(&repo.root) {
        Ok((form, _)) if form.adoptable() => form,
        Ok((form, _)) => return Outcome::Skip(format!("a `{form}` is added to a repository, not one itself")),
        Err(reason) => return Outcome::Skip(format!("lyrn cannot tell the form: {}", first_line(&reason.to_string()))),
    };
    let template = match template::for_form(form) {
        Ok(template) => template,
        Err(reason) => return Outcome::Found(vec![Finding::fail(format!("the `{form}` template cannot be read: {reason}"))]),
    };
    let Kind::Lyrn(native) = &template.kind else {
        unreachable!("a form's template is a lyrn template; `for_form` refuses anything else")
    };

    let files = templates::standard_files(&native.files);
    let missing: Vec<String> = files
        .iter()
        .filter(|f| already_there(&repo.root, Path::new(&f.path)).is_none())
        .map(|f| f.path.clone())
        .collect();

    let behind = match (&facts.standard, native.manifest.standard.as_str()) {
        (Some(theirs), ours) if !ours.is_empty() && crate::standard::order(theirs, ours) == std::cmp::Ordering::Less => Some((theirs.clone(), ours)),
        _ => None,
    };

    if missing.is_empty() {
        if let Some((theirs, ours)) = behind {
            findings.push(
                Finding::warn(format!("on the standard {theirs}; lyrn writes {ours}"))
                    .fix("`lyrn upgrade` merges what changed since into the files, and commits it"),
            );
        }
        return Outcome::from_findings(findings, || format!("all {} files of the `{form}` standard", files.len()));
    }

    let what = if facts.has_lyrn_toml {
        format!(
            "{} of the `{form}` standard missing: {}",
            plural(missing.len(), "file", "files"),
            some_of(&missing, 5)
        )
    } else {
        format!(
            "not a lyrn project yet - no lyrn.toml - and {} of the `{form}` standard missing: {}",
            plural(missing.len(), "file", "files"),
            some_of(&missing, 5)
        )
    };
    findings.push(Finding::fail(what).fix("`lyrn adopt` adds exactly these and touches nothing that is there"));
    Outcome::Found(findings)
}

pub fn check_mark(repo: &Repo) -> Outcome {
    let Ok(facts) = &repo.facts else {
        return Outcome::Skip("the manifests do not parse - `standard` reports it".to_string());
    };
    if !facts.has_lyrn_toml {
        return Outcome::Skip("no lyrn.toml to say".to_string());
    }
    match facts.mark.as_deref() {
        Some("chosen") => Outcome::Pass("a mark of its own".to_string()),
        Some("placeholder") => Outcome::Found(vec![
            Finding::warn("the project wears the line's placeholder mark, not one of its own")
                .fix("choose its mark, put it in assets/, then set `mark = \"chosen\"` in lyrn.toml"),
        ]),
        Some(other) => Outcome::Found(vec![
            Finding::fail(format!("lyrn.toml says `mark = \"{other}\"`, which means nothing")).fix("`placeholder` or `chosen`"),
        ]),
        None => Outcome::Skip("lyrn.toml does not say; it was written before lyrn recorded the mark".to_string()),
    }
}

fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or_default()
}
