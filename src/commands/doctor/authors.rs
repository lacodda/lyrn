//! `authors`: every author in the history is you, or someone `.mailmap` names.
//!
//! A public history is read under a real name. An old alias, a misspelt
//! surname, a laptop that once committed as `user@localhost` - each shows up
//! on every page of the project as somebody else. Pushed commits are not
//! rewritten, so the place to say who is who is `.mailmap`, git's own file for
//! it: an alias mapped to you is you, and anybody else who wrote a commit is
//! named there as themselves. The history is then held to that list, with the
//! global git identity as the one entry nobody has to write down.

use std::collections::BTreeMap;

use super::ai::is_assistant;
use super::git::{Identity, Mailmap, global_config};
use super::{Finding, Outcome, Repo, plural};

pub fn check(repo: &Repo) -> Outcome {
    let Some(git) = &repo.git else {
        return Outcome::Skip("not a git repository".to_string());
    };
    if !git.has_commits() {
        return Outcome::Skip("no commits yet".to_string());
    }

    // A runner's identity, if it has one at all, is a bot's: on CI the
    // history is held to `.mailmap` alone.
    let owner = if repo.on_ci {
        None
    } else {
        match (global_config("user.name"), global_config("user.email")) {
            (Some(name), Some(email)) => Some(Identity::new(name, email)),
            _ => match (git.config("user.name"), git.config("user.email")) {
                (Some(name), Some(email)) => Some(Identity::new(name, email)),
                _ => None,
            },
        }
    };
    let mailmap_text = repo.read_text(".mailmap").unwrap_or_default();
    let mailmap = Mailmap::parse(&mailmap_text);
    if owner.is_none() && mailmap_text.trim().is_empty() {
        return Outcome::Skip("no git identity on this machine and no .mailmap to hold the history to".to_string());
    }

    let log = git
        .run(&[
            "log",
            "--all",
            "--format=%an%x1f%ae%x1f%(trailers:key=Co-authored-by,valueonly,separator=%x1d)%x1e",
        ])
        .unwrap_or_default();
    let history = read_log(&log);
    let report = examine(&history, owner.as_ref(), &mailmap);

    let mut findings = Vec::new();
    for unknown in &report.unknown {
        let what = match (unknown.authored, unknown.co_authored) {
            (a, 0) => format!("`{}` wrote {} and is {}", unknown.who, plural(a, "commit", "commits"), nobody(owner.is_some())),
            (0, c) => format!(
                "`{}` co-authored {} and is {}",
                unknown.who,
                plural(c, "commit", "commits"),
                nobody(owner.is_some())
            ),
            (a, c) => format!(
                "`{}` wrote {} and co-authored {}, and is {}",
                unknown.who,
                plural(a, "commit", "commits"),
                plural(c, "more", "more"),
                nobody(owner.is_some())
            ),
        };
        let fix = match &owner {
            Some(owner) => format!(
                "if it is you, map it in .mailmap: `{owner} {}`\nif it is somebody else, name them there: `{}`",
                unknown.raw, unknown.who
            ),
            None => format!("name them in .mailmap: `{}`, or map the alias to who it is", unknown.who),
        };
        findings.push(Finding::fail(what).fix(fix));
    }

    Outcome::from_findings(findings, || {
        let people = report.people.len();
        match (&owner, people) {
            (Some(owner), 1) if report.people.contains_key(owner) => {
                format!("{}, all by {owner}", plural(history.len(), "commit", "commits"))
            }
            _ => format!(
                "{} by {}, each you or named in .mailmap",
                plural(history.len(), "commit", "commits"),
                plural(people, "person", "people")
            ),
        }
    })
}

fn nobody(have_owner: bool) -> &'static str {
    if have_owner {
        "neither you nor anyone .mailmap names"
    } else {
        "not anyone .mailmap names"
    }
}

/// One commit's people, as recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Commit {
    author: Identity,
    co_authors: Vec<Identity>,
}

fn read_log(log: &str) -> Vec<Commit> {
    log.split('\x1e')
        .filter_map(|record| {
            let record = record.trim_start_matches(['\n', '\r']);
            let mut fields = record.split('\x1f');
            let name = fields.next()?;
            let email = fields.next()?;
            let co_authors = fields.next().unwrap_or_default().split('\x1d').filter_map(Identity::parse).collect();
            Some(Commit {
                author: Identity::new(name.trim(), email.trim()),
                co_authors,
            })
        })
        .collect()
}

#[derive(Debug, Default)]
struct Report {
    /// Everybody the history resolves to who is known, with their commits.
    people: BTreeMap<Identity, usize>,
    unknown: Vec<Unknown>,
}

#[derive(Debug, PartialEq, Eq)]
struct Unknown {
    /// Who the commits resolve to.
    who: Identity,
    /// The first spelling the history has of them, for the `.mailmap` line.
    raw: Identity,
    authored: usize,
    co_authored: usize,
}

fn examine(history: &[Commit], owner: Option<&Identity>, mailmap: &Mailmap) -> Report {
    let mut report = Report::default();
    let mut unknown: BTreeMap<Identity, Unknown> = BTreeMap::new();
    let known = |who: &Identity| owner.is_some_and(|o| o.same_as(who)) || mailmap.names(who);

    for commit in history {
        let people = std::iter::once((&commit.author, true)).chain(
            commit
                .co_authors
                .iter()
                // An assistant named as co-author is a stamp, not a person:
                // `ai-traces` reports it, and no `.mailmap` line should make
                // it somebody.
                .filter(|who| !is_assistant(who))
                .map(|who| (who, false)),
        );
        for (raw, authored) in people {
            let who = mailmap.resolve(raw);
            if known(&who) {
                *report.people.entry(who).or_default() += 1;
                continue;
            }
            let entry = unknown.entry(who.clone()).or_insert_with(|| Unknown {
                who,
                raw: raw.clone(),
                authored: 0,
                co_authored: 0,
            });
            if authored {
                entry.authored += 1;
            } else {
                entry.co_authored += 1;
            }
        }
    }
    let mut unknown: Vec<Unknown> = unknown.into_values().collect();
    unknown.sort_by_key(|u| std::cmp::Reverse(u.authored + u.co_authored));
    report.unknown = unknown;
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    fn who(name: &str, email: &str) -> Identity {
        Identity::new(name, email)
    }

    fn by(author: Identity) -> Commit {
        Commit {
            author,
            co_authors: Vec::new(),
        }
    }

    #[test]
    fn the_log_reads_authors_and_co_authors() {
        let log = "Ada\x1fada@example.com\x1fGrace <grace@example.com>\x1dClaude <noreply@anthropic.com>\x1e\nAda\x1fada@example.com\x1f\x1e\n";
        let history = read_log(log);
        assert_eq!(history.len(), 2);
        assert_eq!(
            history[0].co_authors,
            vec![who("Grace", "grace@example.com"), who("Claude", "noreply@anthropic.com")]
        );
        assert!(history[1].co_authors.is_empty());
    }

    #[test]
    fn a_history_of_one_owner_is_known() {
        let owner = who("Ada Lovelace", "ada@example.com");
        let history = vec![by(owner.clone()), by(who("Ada Lovelace", "ADA@example.com"))];
        let report = examine(&history, Some(&owner), &Mailmap::default());
        assert!(report.unknown.is_empty(), "{:?}", report.unknown);
    }

    #[test]
    fn an_alias_is_unknown_until_mapped() {
        let owner = who("Ada Lovelace", "ada@example.com");
        let history = vec![by(owner.clone()), by(who("lacodda", "old@example.com")), by(who("lacodda", "old@example.com"))];
        let report = examine(&history, Some(&owner), &Mailmap::default());
        assert_eq!(
            report.unknown,
            vec![Unknown {
                who: who("lacodda", "old@example.com"),
                raw: who("lacodda", "old@example.com"),
                authored: 2,
                co_authored: 0
            }]
        );

        let mapped = Mailmap::parse("Ada Lovelace <ada@example.com> lacodda <old@example.com>\n");
        assert!(examine(&history, Some(&owner), &mapped).unknown.is_empty());
    }

    /// A misspelt surname is the case `.mailmap` exists for, and it is not
    /// waved through as "the same address".
    #[test]
    fn a_misspelt_name_at_the_same_address_is_somebody_else() {
        let owner = who("Ada Lovelace", "ada@example.com");
        let history = vec![by(who("Ada Lovlace", "ada@example.com"))];
        assert_eq!(examine(&history, Some(&owner), &Mailmap::default()).unknown.len(), 1);
    }

    #[test]
    fn someone_named_in_the_mailmap_is_known_and_an_assistant_is_left_to_ai_traces() {
        let owner = who("Ada Lovelace", "ada@example.com");
        let mailmap = Mailmap::parse("Grace Hopper <grace@example.com>\n");
        let history = vec![
            by(who("Grace Hopper", "grace@example.com")),
            Commit {
                author: owner.clone(),
                co_authors: vec![who("Claude", "noreply@anthropic.com"), who("Alan Turing", "alan@example.com")],
            },
        ];
        let report = examine(&history, Some(&owner), &mailmap);
        assert_eq!(report.unknown.len(), 1, "{:?}", report.unknown);
        assert_eq!(report.unknown[0].who, who("Alan Turing", "alan@example.com"));
        assert_eq!(report.unknown[0].co_authored, 1);
    }

    #[test]
    fn without_an_owner_the_mailmap_is_the_whole_list() {
        let mailmap = Mailmap::parse("Ada Lovelace <ada@example.com>\n");
        let history = vec![by(who("Ada Lovelace", "ada@example.com")), by(who("Bob", "bob@example.com"))];
        let report = examine(&history, None, &mailmap);
        assert_eq!(report.unknown.len(), 1);
        assert_eq!(report.unknown[0].who, who("Bob", "bob@example.com"));
    }
}
