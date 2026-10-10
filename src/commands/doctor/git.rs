//! The doctor's view of git: the work tree, its history, and who is who.

use std::path::{Path, PathBuf};
use std::process::Command as Process;

/// A work tree, by its top-level directory.
pub struct Git {
    root: PathBuf,
}

impl Git {
    /// The work tree `path` is in, if it is in one and git is installed.
    pub fn open(path: &Path) -> Option<Git> {
        let top = output(Process::new("git").args(["rev-parse", "--show-toplevel"]).current_dir(path))?;
        Some(Git {
            root: PathBuf::from(top.trim()),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// `git <args>` in the work tree; its standard output when it succeeds.
    pub fn run(&self, args: &[&str]) -> Option<String> {
        output(Process::new("git").args(args).current_dir(&self.root))
    }

    /// The tracked files, as `/`-separated paths from the top.
    pub fn tracked_files(&self) -> Vec<String> {
        // `-z` so a path with a space or a non-ASCII name arrives unquoted.
        let listed = self.run(&["ls-files", "-z"]).unwrap_or_default();
        listed.split('\0').filter(|p| !p.is_empty()).map(str::to_string).collect()
    }

    /// Whether there is any commit at all.
    pub fn has_commits(&self) -> bool {
        self.run(&["rev-parse", "--verify", "--quiet", "HEAD"]).is_some()
    }

    /// `git config --get <key>` as it applies here: the repository's own
    /// config over the global one.
    pub fn config(&self, key: &str) -> Option<String> {
        self.run(&["config", "--get", key]).map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
    }

    /// Where the value of `key` that applies here comes from, as git says it:
    /// `file:.git/config`, `file:C:/Users/me/.gitconfig`.
    pub fn config_origin(&self, key: &str) -> Option<String> {
        let line = self.run(&["config", "--show-origin", "--get", key])?;
        line.split('\t').next().map(str::to_string)
    }
}

/// A value from the global config, with the files it includes.
pub fn global_config(key: &str) -> Option<String> {
    output(Process::new("git").args(["config", "--global", "--includes", "--get", key]))
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn output(command: &mut Process) -> Option<String> {
    let out = command.output().ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8(out.stdout).ok()
}

/// A person as git records them: a name and an address.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Identity {
    pub name: String,
    pub email: String,
}

impl Identity {
    pub fn new(name: impl Into<String>, email: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            email: email.into(),
        }
    }

    /// `Name <email>` - also what a trailer carries.
    pub fn parse(text: &str) -> Option<Identity> {
        let text = text.trim();
        let open = text.rfind('<')?;
        let close = text[open..].find('>')? + open;
        let name = text[..open].trim();
        let email = text[open + 1..close].trim();
        (!email.is_empty()).then(|| Identity::new(name, email))
    }

    /// The same person by git's rules: the address regardless of case, the
    /// name exactly - a name spelt differently is exactly what is looked for.
    pub fn same_as(&self, other: &Identity) -> bool {
        self.name == other.name && self.email.eq_ignore_ascii_case(&other.email)
    }
}

impl std::fmt::Display for Identity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} <{}>", self.name, self.email)
    }
}

/// `.mailmap`: who is who in the history.
///
/// Read here rather than left to `git log --use-mailmap`, because the
/// trailers of a commit are not mapped by git and the authors are, and one
/// person has to come out as one person whichever field named them.
#[derive(Debug, Default)]
pub struct Mailmap {
    entries: Vec<Entry>,
}

#[derive(Debug)]
struct Entry {
    proper_name: Option<String>,
    proper_email: Option<String>,
    commit_name: Option<String>,
    commit_email: String,
}

impl Mailmap {
    /// The four forms git documents:
    ///
    /// ```text
    /// Proper Name <commit@email>
    /// <proper@email> <commit@email>
    /// Proper Name <proper@email> <commit@email>
    /// Proper Name <proper@email> Commit Name <commit@email>
    /// ```
    pub fn parse(text: &str) -> Mailmap {
        let mut entries = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut parts: Vec<(Option<String>, String)> = Vec::new();
            let mut rest = line;
            while let Some(open) = rest.find('<') {
                let Some(close) = rest[open..].find('>').map(|c| c + open) else { break };
                let name = rest[..open].trim();
                let email = rest[open + 1..close].trim();
                parts.push(((!name.is_empty()).then(|| name.to_string()), email.to_string()));
                rest = &rest[close + 1..];
                // Anything after the addresses is a comment.
                if parts.len() == 2 {
                    break;
                }
            }
            let entry = match parts.as_slice() {
                [(name, email)] => Entry {
                    proper_name: name.clone(),
                    proper_email: None,
                    commit_name: None,
                    commit_email: email.clone(),
                },
                [(proper_name, proper_email), (commit_name, commit_email)] => Entry {
                    proper_name: proper_name.clone(),
                    proper_email: Some(proper_email.clone()),
                    commit_name: commit_name.clone(),
                    commit_email: commit_email.clone(),
                },
                _ => continue,
            };
            entries.push(entry);
        }
        Mailmap { entries }
    }

    /// Who `identity` is, once the map has been applied. An entry naming the
    /// commit's name as well as its address wins over one naming the address
    /// alone, as in git.
    pub fn resolve(&self, identity: &Identity) -> Identity {
        let matches = |e: &&Entry| e.commit_email.eq_ignore_ascii_case(&identity.email);
        let entry = self
            .entries
            .iter()
            .filter(matches)
            .find(|e| e.commit_name.as_deref().is_some_and(|n| n.eq_ignore_ascii_case(&identity.name)))
            .or_else(|| self.entries.iter().filter(matches).find(|e| e.commit_name.is_none()));
        match entry {
            Some(e) => Identity::new(
                e.proper_name.clone().unwrap_or_else(|| identity.name.clone()),
                e.proper_email.clone().unwrap_or_else(|| identity.email.clone()),
            ),
            None => identity.clone(),
        }
    }

    /// Whether the map names `identity` as somebody: the proper side of an
    /// entry, which is what every alias of a person resolves to.
    pub fn names(&self, identity: &Identity) -> bool {
        self.entries.iter().any(|e| {
            let email = e.proper_email.as_deref().unwrap_or(&e.commit_email);
            email.eq_ignore_ascii_case(&identity.email) && e.proper_name.as_deref().is_none_or(|n| n == identity.name)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn who(name: &str, email: &str) -> Identity {
        Identity::new(name, email)
    }

    #[test]
    fn a_trailer_value_is_a_person() {
        assert_eq!(Identity::parse("Ada Lovelace <ada@example.com>"), Some(who("Ada Lovelace", "ada@example.com")));
        assert_eq!(Identity::parse("  <ada@example.com> "), Some(who("", "ada@example.com")));
        assert_eq!(Identity::parse("Ada Lovelace"), None);
    }

    #[test]
    fn each_form_of_a_mailmap_line_maps_as_git_maps_it() {
        let map = Mailmap::parse(
            "# who is who\n\
             Ada Lovelace <ada@example.com>\n\
             <ada@example.com> <ada@old.example>\n\
             Ada Lovelace <ada@example.com> <aaa@example.com>\n\
             Ada Lovelace <ada@example.com> countess <shared@example.com>  # a comment\n",
        );
        // A name for an address.
        assert_eq!(map.resolve(&who("ada", "ADA@example.com")), who("Ada Lovelace", "ADA@example.com"));
        // An address for an address: the name is kept.
        assert_eq!(map.resolve(&who("A. L.", "ada@old.example")), who("A. L.", "ada@example.com"));
        // Both for an address.
        assert_eq!(map.resolve(&who("whoever", "aaa@example.com")), who("Ada Lovelace", "ada@example.com"));
        // Both for a name at an address - and nobody else at that address.
        assert_eq!(map.resolve(&who("countess", "shared@example.com")), who("Ada Lovelace", "ada@example.com"));
        assert_eq!(map.resolve(&who("someone", "shared@example.com")), who("someone", "shared@example.com"));
    }

    #[test]
    fn the_map_names_the_people_aliases_resolve_to() {
        let map = Mailmap::parse("Ada Lovelace <ada@example.com> lacodda <old@example.com>\nGrace Hopper <grace@example.com>\n");
        assert!(map.names(&who("Ada Lovelace", "ada@example.com")));
        assert!(map.names(&who("Grace Hopper", "GRACE@example.com")));
        // The alias itself is not somebody: it is what the map turns into Ada.
        assert!(!map.names(&who("lacodda", "old@example.com")));
        assert!(!map.names(&who("Grace Hoper", "grace@example.com")));
    }

    #[test]
    fn a_name_spelt_differently_is_another_identity() {
        assert!(who("Ada Lovelace", "ada@example.com").same_as(&who("Ada Lovelace", "ADA@example.com")));
        assert!(!who("Ada Lovelace", "ada@example.com").same_as(&who("Ada Lovlace", "ada@example.com")));
    }
}
