//! The applications of the line that accept plugins.
//!
//! A host is a table entry, not a branch of code: its name, the protocol
//! version it speaks, what a plugin is to it and the line the generated README
//! opens with. Adding the next host is one record plus its fixture.
//!
//! Hosts differ in what a plugin *is*. kilna and kasl run a plugin's command
//! on something they hand over whole; sefy hands a plugin a sealed file to
//! carry somewhere and back. The two are different programs, not one program
//! with different words in it, so the host says which with its [`Shape`] and
//! the form writes the files of that shape.
//!
//! Only hosts that actually accept plugins today are listed. A preset written
//! against a protocol nobody implements cannot be checked by anything - the
//! generated project would compile and its protocol test would pass, because
//! both would be measured against an invention. So `--host turnout` is refused
//! with the reason rather than generating a plugin turnout could never run.

use crate::model::Addon;

/// An application a plugin can be written for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Host {
    /// The host's executable and crate name, e.g. `kilna`.
    pub name: &'static str,
    /// What the host is, for the generated README.
    pub about: &'static str,
    /// The protocol version the host speaks today.
    pub protocol_version: u32,
    /// What a plugin is to this host, and what the host hands it.
    pub shape: Shape,
    /// Where the host looks for plugins, in one line for the README.
    pub lookup: &'static str,
    /// The add-ons a plugin for this host can take: the parts of its protocol
    /// beyond the one call every plugin of its shape answers.
    ///
    /// Empty for a host that asks nothing else. An add-on the form offers and
    /// the host does not take is refused by name, rather than generating code
    /// for requests the host would never send.
    pub addons: &'static [Addon],
}

/// What a plugin is to its host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// The host runs a command the plugin declares on something it hands
    /// over whole - a work, a day - and shows or keeps what comes back.
    Command(Commands),
    /// The host hands the plugin a file to carry somewhere else, or to fetch
    /// back into, and nothing more.
    Transport(Transport),
}

/// A host that runs a plugin's commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Commands {
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
}

/// A host that hands a plugin a file to move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transport {
    /// What the host can ask a transport to do, as the manifest declares them.
    pub operations: &'static [Target],
    /// What a request hands the plugin, in one line for the docs.
    pub request: &'static str,
}

/// One point of extension a host offers, or one operation it asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    /// The value the manifest carries, e.g. `work` or `push`.
    pub key: &'static str,
    /// What it is, for the generated docs.
    pub about: &'static str,
}

/// kilna: the first implementation of the line's plugin convention. Read from
/// kilna's `src-tauri/src/plugin/` rather than from a specification, because
/// the code is what a generated plugin has to satisfy.
const KILNA: Host = Host {
    name: "kilna",
    about: "a desktop workbench for content makers",
    protocol_version: 1,
    shape: Shape::Command(Commands {
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
    }),
    lookup: "the `plugins` directory beside its workspace database, then your `PATH`",
    addons: &[],
};

/// kasl: the second implementation, and the first command-line host. Read
/// from kasl's `src/libs/plugin/` and its "Writing a plugin" page (kasl
/// v1.20). Besides commands it asks requests - an inbox source's issues, and
/// the day's tasks - which `--with source` writes; its events are written by
/// hand, since an event is a moment only the plugin knows what to do with.
const KASL: Host = Host {
    name: "kasl",
    about: "a work activity tracker for the command line",
    protocol_version: 1,
    shape: Shape::Command(Commands {
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
    }),
    lookup: "the `plugins` directory in its data folder (`%LOCALAPPDATA%\\lacodda\\kasl\\plugins` on Windows), then beside its own executable, where the plugins that ship with it live, then your `PATH`",
    addons: &[Addon::Source],
};

/// sefy: a transport, not a command. Read from sefy's `sefy-core/src/plugin/`
/// and its `plugin` reference page (sefy v0.20), with its own two transports -
/// github and sftp - as the working shape. A transport is handed the path of
/// the sealed vault and the name of the remote copy, and never the password,
/// a key or an item.
const SEFY: Host = Host {
    name: "sefy",
    about: "an inconspicuous encrypted store for notes, credentials and files",
    protocol_version: 1,
    shape: Shape::Transport(Transport {
        operations: &[
            Target {
                key: "push",
                about: "read the sealed file at `file` and keep it under `name`",
            },
            Target {
                key: "pull",
                about: "write the copy kept under `name` to `file`",
            },
        ],
        request: "the path of the sealed vault file and the name of its remote copy - never the password, a key or an item",
    }),
    lookup: "the `plugins` directory in its data folder (`%APPDATA%\\sefy\\plugins` on Windows, `~/Library/Application Support/sefy/plugins` on macOS, `$XDG_DATA_HOME/sefy/plugins` on Linux), then your `PATH`",
    addons: &[],
};

/// Every host a plugin can be generated for.
pub const ALL: &[Host] = &[KILNA, KASL, SEFY];

/// The hosts of the line that do not accept plugins yet.
///
/// Named so the refusal can say "not yet" rather than "unknown": a typo and a
/// host whose turn has not come are different mistakes, and the second one
/// tells the reader to come back.
pub const PLANNED: &[&str] = &["turnout"];

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

    /// Whether a plugin for this host can take `addon`, and why not when it
    /// cannot.
    pub fn takes(&self, addon: &str) -> Result<(), String> {
        if self.addons.iter().any(|a| a.as_str() == addon) {
            return Ok(());
        }
        let takers: Vec<&str> = ALL.iter().filter(|h| h.addons.iter().any(|a| a.as_str() == addon)).map(|h| h.name).collect();
        let whose = match takers.as_slice() {
            [] => "no host".to_string(),
            [one] => format!("a {one} plugin"),
            many => format!("a plugin for {}", many.join(" or ")),
        };
        Err(format!(
            "`--with {addon}` is part of the protocol of {whose}; {} asks its plugins nothing beyond the call every plugin answers",
            self.name
        ))
    }
}

impl Commands {
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

    fn commands(host: &Host) -> &Commands {
        match &host.shape {
            Shape::Command(commands) => commands,
            Shape::Transport(_) => panic!("`{}` runs no commands", host.name),
        }
    }

    #[test]
    fn the_hosts_are_the_ones_with_a_running_implementation() {
        // The plan named kasl, turnout and sefy from a registry repository
        // that does not exist. A host is added here when it ships the
        // protocol - kilna in v0.8, kasl in v1.19, sefy in v0.4 - not before.
        let names: Vec<&str> = ALL.iter().map(|host| host.name).collect();
        assert_eq!(names, vec!["kilna", "kasl", "sefy"]);
    }

    #[test]
    fn kasl_offers_a_day_first_and_an_issue() {
        // A generated plugin's one command runs on the first target; a day is
        // what a kasl plugin is most often run on.
        let kasl = find("kasl").unwrap();
        let commands = commands(kasl);
        assert_eq!(commands.primary_target().key, "day");
        assert_eq!(commands.targets.iter().map(|t| t.key).collect::<Vec<_>>(), vec!["day", "issue"]);
        assert_eq!(kasl.prefix(), "kasl-plugin-");
    }

    #[test]
    fn sefy_is_a_transport_that_pushes_and_pulls() {
        // sefy reads a manifest's operations with a closed set: anything but
        // `push` and `pull` makes the whole manifest unreadable.
        let sefy = find("sefy").unwrap();
        let Shape::Transport(transport) = &sefy.shape else {
            panic!("sefy hands a plugin a file, not a subject");
        };
        assert_eq!(transport.operations.iter().map(|o| o.key).collect::<Vec<_>>(), vec!["push", "pull"]);
        assert_eq!(sefy.prefix(), "sefy-plugin-");
        assert!(transport.request.contains("never the password"));
    }

    #[test]
    fn only_kasl_asks_requests_of_a_plugin() {
        let kasl = find("kasl").unwrap();
        assert!(kasl.takes("source").is_ok());

        // kilna and sefy ask nothing beyond the one call: the refusal names
        // who does, so the reader knows the add-on is real and where it goes.
        for name in ["kilna", "sefy"] {
            let refusal = find(name).unwrap().takes("source").unwrap_err();
            assert!(refusal.contains("a kasl plugin"), "{refusal}");
            assert!(refusal.contains(name), "{refusal}");
        }
    }

    #[test]
    fn every_addon_a_host_takes_is_one_the_plugin_form_offers() {
        // A host taking an add-on the form does not offer could never be given
        // it: `--with` is checked against the form first.
        for host in ALL {
            for addon in host.addons {
                assert!(
                    crate::model::Form::Plugin.addons().contains(addon),
                    "`{}` takes `{addon}`, which the plugin form does not offer",
                    host.name
                );
            }
        }
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
    fn every_host_offers_something_to_do() {
        for host in ALL {
            match &host.shape {
                Shape::Command(commands) => assert!(!commands.targets.is_empty(), "`{}` offers no point of extension", host.name),
                Shape::Transport(transport) => assert!(!transport.operations.is_empty(), "`{}` asks for no operation", host.name),
            }
        }
    }

    #[test]
    fn the_prefix_follows_the_line_convention() {
        assert_eq!(KILNA.prefix(), "kilna-plugin-");
    }
}
