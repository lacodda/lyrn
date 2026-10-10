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
    let facts = match &repo.facts {
        Ok(facts) => facts,
        Err(reason) => return Outcome::Found(vec![Finding::fail(reason.clone()).fix("fix the manifest; nothing can read it as it is")]),
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
    if missing.is_empty() {
        let revision = match (&facts.standard, native.manifest.standard.as_str()) {
            (Some(theirs), ours) if theirs != ours && !ours.is_empty() => format!("; lyrn.toml says {theirs}, lyrn writes {ours}"),
            _ => String::new(),
        };
        return Outcome::Pass(format!("all {} files of the `{form}` standard{revision}", files.len()));
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
    Outcome::Found(vec![
        Finding::fail(what).fix("`lyrn adopt` adds exactly these and touches nothing that is there"),
    ])
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
