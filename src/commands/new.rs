use std::error::Error;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::Command as Process;

use crate::cli::NewArgs;
use crate::generate;
use crate::host;
use crate::model::{Context, Form, Placement, TemplateManifest};
use crate::naming::{self, PLACEHOLDER_ACCENT};
use crate::templates;

/// Build the context a generation runs with.
///
/// Everything can be given on the command line; the wizard only fills what is
/// still missing, and only when there is a terminal to ask into.
pub fn build_context(args: &NewArgs, manifest: &TemplateManifest, interactive: bool) -> Result<Context, Box<dyn Error>> {
    naming::validate_name(&args.name)?;

    // An add-on this form does not have would otherwise be accepted and write
    // nothing: the command succeeds, and the thing that was asked for is
    // simply absent. Clap only checks that the name exists at all.
    for addon in &args.with {
        if !args.form.addons().contains(addon) {
            let known = args.form.addons().iter().map(|a| a.as_str()).collect::<Vec<_>>();
            let offer = if known.is_empty() {
                "it has none".to_string()
            } else {
                format!("it has: {}", known.join(", "))
            };
            return Err(format!("the `{}` form has no `{addon}` add-on - {offer}", args.form).into());
        }
    }

    // `--host` given to a form that has no host would be accepted and ignored:
    // the generated project would be right about everything except the one
    // thing the flag was for.
    let host = match (&args.host, args.form.takes_a_host()) {
        (Some(name), true) => Some(host::find(name)?),
        (Some(name), false) => {
            return Err(format!("the `{}` form is not generated against a host, so `--host {name}` means nothing", args.form).into());
        }
        (None, true) => {
            return Err(format!(
                "`--form plugin` needs the application the plugin extends: `--host {}` (see `lyrn forms`)",
                host::ALL.first().map(|h| h.name).unwrap_or("HOST")
            )
            .into());
        }
        (None, false) => None,
    };

    let accent = match &args.accent {
        Some(value) => naming::resolve_accent(value)?,
        None if interactive => prompt_accent()?,
        None => PLACEHOLDER_ACCENT.to_string(),
    };

    // Whether this project still wears the line's placeholder rather than a
    // mark of its own. Written into `lyrn.toml` as a fact rather than left for
    // `lyrn doctor` to infer: the alternative is the doctor comparing icon
    // bytes against a copy of the placeholder it carries, which goes stale the
    // first time the placeholder is redrawn and then reports every project as
    // branded. The generator knows the answer; it says so.
    let mark_chosen = accent != PLACEHOLDER_ACCENT;

    // A documentation site describes a product that already exists, so its
    // default says that rather than calling the site "a docs".
    let default_description = match args.form {
        Form::Docs => format!("The documentation of {}.", naming::title_from_name(&args.name)),
        form => format!("A {form} on the lacodda line's stack."),
    };
    let description = match &args.description {
        Some(value) => value.clone(),
        None if interactive => prompt_line("What is it, in one line?", &default_description)?,
        None => default_description,
    };

    let author = match &args.author {
        Some(value) => value.clone(),
        None => git_config("user.name").unwrap_or_else(|| "The author".to_string()),
    };

    // Resolved once: the lookup can spawn `gh`, and it is asked for twice.
    let repo = repo(args);
    let owner = repo.split('/').next().unwrap_or("OWNER").to_string();
    // A github.io project site is served under the repository's name, which
    // need not be the project's.
    let repo_name = repo.split('/').nth(1).unwrap_or(&args.name).to_string();
    let title = naming::title_from_name(&args.name);

    let mut context = Context::new();
    context
        .set("name", &args.name)
        .set("title_json", naming::json_string(&title))
        .set("title", title)
        .set("description_json", naming::json_string(&description))
        .set("description", description)
        .set("repo_name", repo_name)
        .set("accent", accent)
        .set("mark", if mark_chosen { "chosen" } else { "placeholder" })
        .set("author", author)
        .set("form", args.form.as_str())
        .set("year", current_year())
        .set("date", current_date())
        .set("registry", "https://lacodda.github.io/dowel/r")
        .set("lyrn_version", env!("CARGO_PKG_VERSION"))
        .set("standard", manifest.standard.clone())
        // The repository the generated project will live in. Guessed from the
        // git identity so the installers and the update check point somewhere
        // real; `--repo` overrides it.
        .set("repo", &repo)
        // The owner alone, for a bundle identifier like `com.owner.my_app`.
        .set("owner", owner)
        // A Rust crate name cannot contain a hyphen where it is used as an
        // identifier, which the Tauri shell does: `my_app::run()`.
        .set("lib_name", args.name.replace('-', "_"))
        // Environment variables are shouted and hyphen-free: `MY_TOOL_VERSION`.
        .set("env_prefix", args.name.to_uppercase().replace('-', "_"))
        // Measured, not assumed: a number that is wrong is worse than absent,
        // because `cargo install` believes it.
        .set("msrv", msrv());

    // The Tauri plugin's Rust type, e.g. `WordCount`, and the single command
    // both halves agree on. Derived from the name rather than asked for: a
    // scaffold has exactly one command, and naming it separately would be a
    // second thing to keep in step with nothing gained.
    if args.form == Form::TauriPlugin {
        // `version` rather than `describe`: the command's name is imported
        // into the webview test beside vitest's own globals, and `describe` is
        // one of them - the generated test failed to parse at all. A scaffold
        // that does not build is worse than one that is dull.
        const COMMAND: &str = "version";

        context
            .set("type_name", naming::type_from_name(&args.name))
            .set("command_key", COMMAND)
            .set("command_fn", COMMAND)
            .set("command_camel", naming::camel_from_key(COMMAND))
            .set("bin_name", format!("tauri-plugin-{}", args.name));
    }

    if let Some(host) = host {
        let target = host.primary_target();
        context
            .set("host", host.name)
            .set("prefix", host.prefix())
            .set("host_about", host.about)
            .set("host_lookup", host.lookup)
            .set("protocol_version", host.protocol_version.to_string())
            .set("subject_about", host.subject)
            .set("target", target.key)
            .set("target_about", target.about)
            // Every point the host offers, for the generated protocol test to
            // hold a command against. Written out as the Rust literal the test
            // reads, so the test needs no parsing of its own.
            .set(
                "target_list",
                host.targets.iter().map(|t| format!("\"{}\"", t.key)).collect::<Vec<_>>().join(", "),
            )
            .set("command_key", "run")
            .set("command_label", format!("Run {}", naming::title_from_name(&args.name)))
            .set("bin_name", format!("{}{}", host.prefix(), args.name));
    }

    Ok(context)
}

pub fn run(args: NewArgs) -> Result<(), Box<dyn Error>> {
    let manifest: TemplateManifest = toml::from_str(templates::manifest_for(args.form))?;
    let interactive = !args.assume_yes && std::io::stdin().is_terminal();

    let context = build_context(&args, &manifest, interactive)?;
    // A new project gets a directory named after it; a form that adds to a
    // repository adds to the one it is run in.
    let root = args.path.clone().unwrap_or_else(|| match args.form.placement() {
        Placement::NewDirectory => PathBuf::from(&args.name),
        Placement::IntoExisting => PathBuf::from("."),
    });

    let plan = generate::plan_with(&templates::sources_for(args.form), &manifest, &context, &args.with)?;

    // Checked before a dry run too: a dry run that promises files the real
    // run would refuse to write is a promise the tool then breaks.
    match args.form.placement() {
        Placement::NewDirectory => generate::check_destination(&root)?,
        Placement::IntoExisting => generate::check_additions(&plan, &root, templates::foreign_sites_for(args.form))?,
    }

    let verb = match args.form.placement() {
        Placement::NewDirectory => "create",
        Placement::IntoExisting => "add",
    };
    if args.dry_run {
        println!("Would {verb} {} in `{}`:\n", plural(plan.files.len()), root.display());
        println!("{}", indent(&plan.tree()));
        return Ok(());
    }

    // With someone at the terminal, the tree is shown before anything is
    // written, and nothing is written until they say so: a form or an add-on
    // that brings more (or less) than was expected is cheapest to notice
    // here, before it is files on disk and a first commit.
    if interactive {
        println!("\nWill {verb} {} in `{}`:\n", plural(plan.files.len()), root.display());
        println!("{}\n", indent(&plan.tree()));
        if !confirm(&format!("{} them?", capitalise(verb)))? {
            println!("Nothing was written.");
            return Ok(());
        }
    }

    generate::write(&plan, &root)?;
    let done = match args.form.placement() {
        Placement::NewDirectory => "Created",
        Placement::IntoExisting => "Added",
    };
    println!("{done} {} in `{}`.", plural(plan.files.len()), root.display());

    if !args.no_hooks {
        run_hooks(&manifest, &root)?;
    }

    print_next_steps(&args, &root);
    Ok(())
}

fn run_hooks(manifest: &TemplateManifest, root: &std::path::Path) -> Result<(), Box<dyn Error>> {
    for hook in &manifest.hooks {
        let Some((program, rest)) = hook.run.split_first() else { continue };
        print!("{}... ", hook.name);
        use std::io::Write;
        std::io::stdout().flush().ok();

        // A hook's own chatter would land in the middle of the line this
        // function is writing; what matters here is whether it worked.
        let dir = hook.dir.as_deref().map_or_else(|| root.to_path_buf(), |d| root.join(d));
        let outcome = Process::new(program).args(rest).current_dir(dir).output();
        match outcome {
            Ok(out) if out.status.success() => println!("done"),
            Ok(out) => {
                println!("failed");
                if !hook.optional {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    return Err(format!("`{}` failed: {}", hook.run.join(" "), stderr.trim()).into());
                }
            }
            Err(e) if hook.optional => {
                // A missing tool is not a failed generation: the files are
                // already on disk and the step is one command away.
                println!("skipped ({e})");
            }
            Err(e) => return Err(Box::new(e)),
        }
    }
    Ok(())
}

fn print_next_steps(args: &NewArgs, root: &std::path::Path) {
    println!("\nNext:");
    match args.form.placement() {
        Placement::NewDirectory => println!("  cd {}", root.display()),
        // The site is a project inside the repository; its commands run there.
        // `Path::join` keeps a leading `./`, which reads as noise here.
        Placement::IntoExisting if root == std::path::Path::new(".") => println!("  cd docs"),
        Placement::IntoExisting => println!("  cd {}", root.join("docs").display().to_string().replace('\\', "/")),
    }
    match args.form {
        Form::Docs => {
            if args.no_hooks {
                println!("  pnpm install");
            }
            println!("  pnpm dev");
            // Pages is off in a new repository, and the workflow's deploy job
            // fails until it is on; saying so here saves the first red run.
            println!("\nTo publish: Settings -> Pages -> Source: GitHub Actions, then push to main.");
        }
        Form::Spa => {
            if args.no_hooks {
                println!("  pnpm install");
            }
            println!("  pnpm dev");
        }
        Form::Cli => {
            println!("  cargo run -- hello");
        }
        Form::Mono => {
            if args.no_hooks {
                println!("  pnpm install");
            }
            // The package is what ships, so building it is the first thing
            // worth seeing; the stand has nothing to render until it exists.
            println!("  pnpm build");
            if args.with.contains(&crate::model::Addon::Stand) {
                println!("  pnpm stand");
            }
        }
        Form::Workspace => {
            // Named, because a workspace has more than one binary target the
            // moment anyone adds a second crate, and `cargo run` then refuses.
            println!("  cargo run --package {} -- hello", args.name);
        }
        Form::Desktop => {
            if args.no_hooks {
                println!("  pnpm install");
            }
            println!("  pnpm tauri dev");
        }
        Form::Service => {
            println!("  docker compose -f deploy/compose.yml up -d db");
            println!("  cargo run");
        }
        Form::Plugin => {
            // The protocol test is what says the plugin is one, so it is the
            // first thing worth running - before the binary is put anywhere.
            println!("  cargo test");
            if let Some(host) = &args.host {
                println!("  cargo run -- --manifest    # what {host} will read");
            }
        }
        Form::TauriPlugin => {
            if args.no_hooks {
                println!("  pnpm install");
            }
            // Both halves, because a plugin whose halves disagree passes
            // either gate alone.
            println!("  cargo test");
            println!("  pnpm test");
        }
    }
}

fn plural(count: usize) -> String {
    if count == 1 { "1 file".to_string() } else { format!("{count} files") }
}

fn indent(text: &str) -> String {
    text.lines().map(|line| format!("  {line}")).collect::<Vec<_>>().join("\n")
}

/// Where the generated project will live: `owner/name`.
///
/// The owner is looked up rather than derived: a GitHub account name is not a
/// person's name, and turning "Jane Smith" into `jane-smith` produces a URL
/// that looks right and resolves to nobody. `git config github.user` first,
/// then the authenticated `gh` account, and failing both an obvious
/// placeholder - a name that is visibly a blank is safer than a plausible one.
fn repo(args: &NewArgs) -> String {
    if let Some(explicit) = &args.repo {
        return explicit.clone();
    }
    let owner = git_config("github.user").or_else(gh_login).unwrap_or_else(|| "OWNER".to_string());
    format!("{owner}/{}", args.name)
}

/// The GitHub account `gh` is logged in as, if it is installed and logged in.
fn gh_login() -> Option<String> {
    let output = Process::new("gh").args(["api", "user", "--jq", ".login"]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let login = String::from_utf8(output.stdout).ok()?.trim().to_string();
    if login.is_empty() { None } else { Some(login) }
}

/// The Rust version the generated project declares as its minimum.
///
/// Read from the toolchain that is generating it: the edition the template
/// uses already sets a floor, and claiming a lower number than the compiler in
/// the room would be a promise nobody has tested. The CI job the template
/// ships reads this back out of the manifest and builds against it, so a wrong
/// number fails rather than rots.
fn msrv() -> String {
    // `rustc 1.98.0 (88d9e12ae 2026-08-18)` - the second word is the version.
    let output = Process::new("rustc").arg("--version").output().ok();
    let measured = output
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|line| line.split_whitespace().nth(1).map(str::to_string))
        .and_then(|v| {
            let mut parts = v.split('.');
            match (parts.next(), parts.next()) {
                (Some(major), Some(minor)) => Some(format!("{major}.{minor}")),
                _ => None,
            }
        });
    // Edition 2024 needs 1.85; without a toolchain to ask, say the floor
    // rather than inventing a number.
    measured.unwrap_or_else(|| "1.85".to_string())
}

fn git_config(key: &str) -> Option<String> {
    let output = Process::new("git").args(["config", "--get", key]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?.trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}

fn prompt_accent() -> Result<String, Box<dyn Error>> {
    use dialoguer::{Input, theme::ColorfulTheme};
    let raw: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Accent (a product of the line, or #rrggbb)")
        .default(PLACEHOLDER_ACCENT.to_string())
        .interact_text()?;
    Ok(naming::resolve_accent(&raw)?)
}

fn confirm(prompt: &str) -> Result<bool, Box<dyn Error>> {
    use dialoguer::{Confirm, theme::ColorfulTheme};
    Ok(Confirm::with_theme(&ColorfulTheme::default()).with_prompt(prompt).default(true).interact()?)
}

fn capitalise(word: &str) -> String {
    let mut chars = word.chars();
    chars.next().map(|first| first.to_uppercase().chain(chars).collect()).unwrap_or_default()
}

fn prompt_line(prompt: &str, default: &str) -> Result<String, Box<dyn Error>> {
    use dialoguer::{Input, theme::ColorfulTheme};
    let value: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .default(default.to_string())
        .interact_text()?;
    Ok(value)
}

fn current_year() -> String {
    chrono::Local::now().format("%Y").to_string()
}

fn current_date() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(name: &str) -> NewArgs {
        NewArgs {
            name: name.to_string(),
            form: Form::Spa,
            host: None,
            accent: None,
            description: None,
            author: Some("Tester".to_string()),
            path: None,
            repo: Some("tester/demo-app".to_string()),
            with: Vec::new(),
            assume_yes: true,
            dry_run: false,
            no_hooks: true,
        }
    }

    #[test]
    fn a_non_interactive_run_needs_no_answers() {
        let manifest = TemplateManifest::default();
        let context = build_context(&args("demo-app"), &manifest, false).unwrap();
        assert_eq!(context.get("name"), Some("demo-app"));
        assert_eq!(context.get("title"), Some("Demo App"));
        assert_eq!(context.get("accent"), Some(PLACEHOLDER_ACCENT));
    }

    #[test]
    fn a_bad_name_is_refused_before_anything_is_written() {
        let manifest = TemplateManifest::default();
        assert!(build_context(&args("Demo App"), &manifest, false).is_err());
    }

    #[test]
    fn an_accent_names_a_product_of_the_line() {
        let manifest = TemplateManifest::default();
        let mut a = args("demo-app");
        a.accent = Some("kilna".to_string());
        let context = build_context(&a, &manifest, false).unwrap();
        assert_eq!(context.get("accent"), Some("#D9569E"));
    }

    /// A project generated without an accent still wears the umbrella mark,
    /// and `lyrn.toml` says so. The doctor reads this rather than comparing
    /// the icon against a copy of the placeholder it would have to carry.
    #[test]
    fn a_project_without_an_accent_is_recorded_as_unmarked() {
        let manifest = TemplateManifest::default();
        let context = build_context(&args("demo-app"), &manifest, false).unwrap();
        assert_eq!(context.get("mark"), Some("placeholder"));
    }

    /// Naming a colour is what choosing a mark looks like from here: the
    /// accent is the one thing `lyrn new` learns about a product's identity,
    /// and dowel derives the rest of the palette from it.
    #[test]
    fn choosing_an_accent_is_choosing_a_mark() {
        let manifest = TemplateManifest::default();
        let mut a = args("demo-app");
        a.accent = Some("kilna".to_string());
        assert_eq!(build_context(&a, &manifest, false).unwrap().get("mark"), Some("chosen"));

        // A literal colour counts too - a product may have a mark before it
        // has a place in the line's registry.
        let mut b = args("demo-app");
        b.accent = Some("#123456".to_string());
        assert_eq!(build_context(&b, &manifest, false).unwrap().get("mark"), Some("chosen"));
    }

    /// The placeholder's own hex, given explicitly, is still the placeholder.
    /// Otherwise `--accent '#6E7079'` would silently promote an unmarked
    /// project to a marked one, and the doctor would stop asking.
    #[test]
    fn spelling_out_the_placeholder_colour_is_not_choosing_a_mark() {
        let manifest = TemplateManifest::default();
        let mut a = args("demo-app");
        a.accent = Some(PLACEHOLDER_ACCENT.to_string());
        assert_eq!(build_context(&a, &manifest, false).unwrap().get("mark"), Some("placeholder"));
    }

    #[test]
    fn an_unknown_accent_is_refused() {
        let manifest = TemplateManifest::default();
        let mut a = args("demo-app");
        a.accent = Some("chartreuse".to_string());
        assert!(build_context(&a, &manifest, false).is_err());
    }

    #[test]
    fn the_standard_comes_from_the_manifest() {
        let manifest = TemplateManifest {
            standard: "2026.09".to_string(),
            ..Default::default()
        };
        let context = build_context(&args("demo-app"), &manifest, false).unwrap();
        assert_eq!(context.get("standard"), Some("2026.09"));
    }
}
