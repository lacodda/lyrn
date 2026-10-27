//! The applications of the line that accept plugins.
//!
//! A host is a table entry, not a branch of code: its name, the protocol
//! version it speaks, the points a plugin may extend and the line the
//! generated README opens with. Adding the next host is one record plus its
//! fixture.
//!
//! Only hosts that actually accept plugins today are listed. A preset written
//! against a protocol nobody implements cannot be checked by anything - the
//! generated project would compile and its protocol test would pass, because
//! both would be measured against an invention. So `--host turnout` is refused
//! with the reason rather than generating a plugin turnout could never run.

/// An application a plugin can be written for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Host {
    /// The host's executable and crate name, e.g. `kilna`.
    pub name: &'static str,
    /// What the host is, for the generated README.
    pub about: &'static str,
    /// The protocol version the host speaks today.
    pub protocol_version: u32,
    /// The points a plugin may extend, in the order the host lists them.
    ///
    /// The first is what the generated command targets: a form has to pick
    /// one, and the first is the host's own most common case.
    pub targets: &'static [Target],
    /// What the host hands a plugin under `subject`, in one line for the docs.
    pub subject: &'static str,
    /// What the host does with the `meta` a reply carries, as the generated
    /// docs finish the sentence "fields under `meta` are ...".
    ///
    /// Hosts differ here and the plugin's author has to know which: one keeps
    /// the fields, another only shows them, and a plugin written for the first
    /// would wait in vain on the second for its fields to come back.
    pub meta: &'static str,
    /// Where the host looks for plugins, in one line for the README.
    pub lookup: &'static str,
}

/// One point of extension a host offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    /// The value the manifest carries, e.g. `work`.
    pub key: &'static str,
    /// What it is, for the generated docs.
    pub about: &'static str,
}

/// kilna: the first and so far only implementation of the line's plugin
/// convention. Read from kilna's `src-tauri/src/plugin/` rather than from a
/// specification, because the code is what a generated plugin has to satisfy.
const KILNA: Host = Host {
    name: "kilna",
    about: "a desktop workbench for content makers",
    protocol_version: 1,
    targets: &[
        Target {
            key: "work",
            about: "a work: the reply's `meta` is merged into the work's fields",
        },
        Target {
            key: "release",
            about: "a release: the reply's `meta` is merged into the release's fields",
        },
    ],
    subject: "the row the command was invoked on, plus a `bodies` object holding each version role's latest text",
    meta: "merged into the subject's fields - the host adds and overwrites these keys and never clears the rest",
    lookup: "the `plugins` directory beside its workspace database, then your `PATH`",
};

/// kasl: the second implementation, and the first command-line host. Read
/// from kasl's `src/libs/plugin/` and its "Writing a plugin" page (kasl
/// v1.19). Its plugins also hear events and answer inbox requests; those are
/// written by hand, since a form generates the one command every plugin has.
const KASL: Host = Host {
    name: "kasl",
    about: "a work activity tracker for the command line",
    protocol_version: 1,
    targets: &[
        Target {
            key: "day",
            about: "a day, the one `--date` names (today by default)",
        },
        Target {
            key: "issue",
            about: "an inbox issue, the one its key names",
        },
    ],
    subject: "the day as `kasl report --json` prints it, or the issue as `kasl inbox show --json` prints it",
    meta: "shown under the plugin's message - the host keeps no field a plugin writes",
    lookup: "the `plugins` directory in its data folder (`%LOCALAPPDATA%\\lacodda\\kasl\\plugins` on Windows), then your `PATH`",
};

/// Every host a plugin can be generated for.
pub const ALL: &[Host] = &[KILNA, KASL];

/// The hosts of the line that do not accept plugins yet.
///
/// Named so the refusal can say "not yet" rather than "unknown": a typo and a
/// host whose turn has not come are different mistakes, and the second one
/// tells the reader to come back.
pub const PLANNED: &[&str] = &["sefy", "turnout"];

/// Look a host up by name.
pub fn find(name: &str) -> Result<&'static Host, String> {
    if let Some(host) = ALL.iter().find(|host| host.name == name) {
        return Ok(host);
    }

    if PLANNED.contains(&name) {
        return Err(format!(
            "`{name}` does not accept plugins yet; when it does it will be listed here (known: {})",
            known()
        ));
    }

    Err(format!("unknown host `{name}` (known: {})", known()))
}

fn known() -> String {
    ALL.iter().map(|host| host.name).collect::<Vec<_>>().join(", ")
}

impl Host {
    /// The prefix an executable must carry to be discovered by this host.
    pub fn prefix(&self) -> String {
        format!("{}-plugin-", self.name)
    }

    /// The target a generated plugin's single command is offered on.
    pub fn primary_target(&self) -> &'static Target {
        // Every host in the table declares at least one; the test below holds
        // that, so the fallback would be unreachable rather than defensive.
        &self.targets[0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hosts_are_the_ones_with_a_running_implementation() {
        // The plan named kasl, turnout and sefy from a registry repository
        // that does not exist. A host is added here when it ships the
        // protocol - kilna in v0.8, kasl in v1.19 - not before.
        let names: Vec<&str> = ALL.iter().map(|host| host.name).collect();
        assert_eq!(names, vec!["kilna", "kasl"]);
    }

    #[test]
    fn kasl_offers_a_day_first_and_an_issue() {
        // A generated plugin's one command runs on the first target; a day is
        // what a kasl plugin is most often run on.
        let kasl = find("kasl").unwrap();
        assert_eq!(kasl.primary_target().key, "day");
        assert_eq!(kasl.targets.iter().map(|t| t.key).collect::<Vec<_>>(), vec!["day", "issue"]);
        assert_eq!(kasl.prefix(), "kasl-plugin-");
    }

    #[test]
    fn a_host_that_has_not_arrived_is_refused_differently_from_a_typo() {
        let waiting = find("turnout").unwrap_err();
        let typo = find("kilnaa").unwrap_err();

        assert!(waiting.contains("does not accept plugins yet"), "{waiting}");
        assert!(typo.contains("unknown host"), "{typo}");
    }

    #[test]
    fn no_host_is_both_listed_and_planned() {
        // Leaving a name in `PLANNED` after its host arrives would answer
        // "not yet" about something that works.
        for host in ALL {
            assert!(!PLANNED.contains(&host.name), "`{}` is listed as a host and as planned", host.name);
        }
    }

    #[test]
    fn every_host_offers_at_least_one_target() {
        for host in ALL {
            assert!(!host.targets.is_empty(), "`{}` offers no point of extension", host.name);
        }
    }

    #[test]
    fn the_prefix_follows_the_line_convention() {
        assert_eq!(KILNA.prefix(), "kilna-plugin-");
    }
}
