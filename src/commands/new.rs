use std::error::Error;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::Command as Process;

use crate::cli::{IdentityArgs, NewArgs, ProjectArgs};
use crate::generate::{self, GenerateError, Plan};
use crate::host;
use crate::model::{Context, Form, Hook, Placement};
use crate::naming::{self, PLACEHOLDER_ACCENT};
use crate::template::cargo_generate::{Answers, Facts};
use crate::template::{self, Kind, Native, Origin, Template};
use crate::templates;

/// Everything a generation is asked for, whichever command asked.
pub struct Wanted<'a> {
    pub name: &'a str,
    pub form: Form,
    pub host: Option<&'a str>,
    pub with: &'a [String],
    pub identity: &'a IdentityArgs,
}

/// Build the context a generation runs with.
///
/// Everything can be given on the command line; the wizard only fills what is
/// still missing, and only when there is a terminal to ask into.
pub fn build_context(wanted: &Wanted, native: &Native, origin: &Origin, interactive: bool) -> Result<Context, Box<dyn Error>> {
    let args = wanted;
    let identity = wanted.identity;
    naming::validate_name(args.name)?;

    // An add-on this template does not have would otherwise be accepted and
    // write nothing: the command succeeds, and the thing that was asked for
    // is simply absent.
    for addon in args.with {
        if !native.addons.iter().any(|a| a.name == *addon) {
            let known = native.addons.iter().map(|a| a.name.as_str()).collect::<Vec<_>>();
            let offer = if known.is_empty() {
                "it has none".to_string()
            } else {
                format!("it has: {}", known.join(", "))
            };
            let whose = match origin {
                Origin::BuiltIn => format!("the `{}` form", args.form),
                _ => "the template".to_string(),
            };
            return Err(format!("{whose} has no `{addon}` add-on - {offer}").into());
        }
    }

    // `--host` given to a form that has no host would be accepted and ignored:
    // the generated project would be right about everything except the one
    // thing the flag was for.
    let host = match (args.host, args.form.takes_a_host()) {
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

    let accent = match &identity.accent {
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
        Form::Docs => format!("The documentation of {}.", naming::title_from_name(args.name)),
        form => format!("A {form} on the lacodda line's stack."),
    };
    let description = match &identity.description {
        Some(value) => value.clone(),
        None if interactive => prompt_line("What is it, in one line?", &default_description)?,
        None => default_description,
    };
    let description = naming::one_line(&description);

    let author = match &identity.author {
        Some(value) => value.clone(),
        None => git_config("user.name").unwrap_or_else(|| "The author".to_string()),
    };

    // Resolved once: the lookup can spawn `gh`, and it is asked for twice.
    let repo = repo(args.name, identity.repo.as_deref());
    let owner = repo.split('/').next().unwrap_or("OWNER").to_string();
    // A github.io project site is served under the repository's name, which
    // need not be the project's.
    let repo_name = repo.split('/').nth(1).unwrap_or(args.name).to_string();
    let title = naming::title_from_name(args.name);

    let mut context = Context::new();
    context
        .set("name", args.name)
        .set("title_json", naming::json_string(&title))
        .set("title", title)
        // The description is the one free text a template pastes in, and each
        // place it lands has its own quoting: JSON (which TOML and YAML also
        // read), a Rust literal, HTML and JSX text, a block comment. The bare
        // value is for prose - Markdown, a `///` line.
        .set("description_json", naming::json_string(&description))
        .set("description_rust", naming::rust_string(&description))
        .set("description_html", naming::html_text(&description))
        .set("description_comment", naming::comment_text(&description))
        // The workspace's library crate describes itself by the project's.
        .set(
            "core_description_json",
            naming::json_string(&format!("Core library for {}: {description}", args.name)),
        )
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
        .set("standard", native.manifest.standard.clone())
        // Where the files came from and at which revision: `lyrn.toml` keeps
        // both, so the template a project was made from can be named again -
        // and `lyrn upgrade` can tell what changed since.
        .set("template_source", origin.source())
        .set("template_revision", origin.revision())
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
            .set("type_name", naming::type_from_name(args.name))
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
            .set("command_label", format!("Run {}", naming::title_from_name(args.name)))
            .set("bin_name", format!("{}{}", host.prefix(), args.name));
    }

    Ok(context)
}

/// The template a project is generated from: the one `--template` names, or
/// the form's - yours under ~/.lyrn/templates if you have one.
pub fn template_for(project: &ProjectArgs) -> Result<Template, Box<dyn Error>> {
    match &project.template {
        Some(spec) => template::resolve(spec),
        None => template::for_form(project.form.unwrap_or(Form::Spa)),
    }
}

/// Where a template's files go when nothing says otherwise.
pub fn placement_of(template: &Template) -> Placement {
    match &template.kind {
        Kind::Lyrn(native) => native.form.placement(),
        Kind::CargoGenerate(_) => Placement::NewDirectory,
    }
}

pub fn run(args: NewArgs) -> Result<(), Box<dyn Error>> {
    let template = template_for(&args.project)?;
    // A new project gets a directory named after it; a form that adds to a
    // repository adds to the one it is run in.
    let placement = placement_of(&template);
    let root = args.path.clone().unwrap_or_else(|| match placement {
        Placement::NewDirectory => PathBuf::from(&args.name),
        Placement::IntoExisting => PathBuf::from("."),
    });
    generate_project(&args.name, &template, &args.project, &root, placement)
}

/// What a generation is about to do, whichever kind of template planned it.
struct Planned {
    plan: Plan,
    hooks: Vec<Hook>,
    /// The form, for a lyrn template; a cargo-generate one has none.
    form: Option<Form>,
}

/// Plan a project, check the destination, show or write it, and run its
/// hooks. `placement` is where the files go this time: `init` puts any form
/// into a directory that already exists.
pub fn generate_project(name: &str, template: &Template, project: &ProjectArgs, root: &Path, placement: Placement) -> Result<(), Box<dyn Error>> {
    let interactive = !project.assume_yes && std::io::stdin().is_terminal();
    let form = match &template.kind {
        Kind::Lyrn(native) => Some(native.form),
        Kind::CargoGenerate(_) => None,
    };
    if let Some(notice) = template.origin.notice(form) {
        println!("{notice}");
    }

    let planned = match &template.kind {
        Kind::Lyrn(native) => {
            if !project.define.is_empty() {
                return Err("`--define` answers a cargo-generate template's placeholders; this is a lyrn template, which asks with its own flags".into());
            }
            let wanted = Wanted {
                name,
                form: native.form,
                host: project.host.as_deref(),
                with: &project.with,
                identity: &project.identity,
            };
            let context = build_context(&wanted, native, &template.origin, interactive)?;
            Planned {
                plan: generate::plan_with(&native.files, &native.manifest, &context, &project.with)?,
                hooks: native.manifest.hooks.clone(),
                form: Some(native.form),
            }
        }
        Kind::CargoGenerate(cargo) => {
            refuse_what_cargo_generate_ignores(project)?;
            naming::validate_name(name)?;
            let username = project.identity.author.clone().or_else(|| git_config("user.name")).unwrap_or_default();
            let authors = match (project.identity.author.is_some(), git_config("user.email")) {
                (false, Some(email)) if !username.is_empty() => format!("{username} <{email}>"),
                _ => username.clone(),
            };
            let facts = Facts {
                authors,
                username,
                is_init: placement == Placement::IntoExisting,
                within_cargo_project: root.ancestors().skip(1).any(|a| a.join("Cargo.toml").is_file()),
            };
            let defines = parse_defines(&project.define)?;
            Planned {
                plan: cargo.plan(
                    name,
                    &facts,
                    &Answers {
                        defines: &defines,
                        interactive,
                    },
                )?,
                hooks: cargo.hooks(),
                form: None,
            }
        }
    };
    let plan = &planned.plan;

    // Checked before a dry run too: a dry run that promises files the real
    // run would refuse to write is a promise the tool then breaks.
    match placement {
        Placement::NewDirectory => generate::check_destination(root)?,
        Placement::IntoExisting => {
            let foreign = planned.form.map(templates::foreign_sites_for).unwrap_or(&[]);
            generate::check_additions(plan, root, foreign).map_err(|e| with_the_other_way(e, planned.form))?
        }
    }

    // The words follow what the form is, not where it goes: a project started
    // in place is still created, and a documentation site is still added.
    let adds = planned.form.is_some_and(|f| f.placement() == Placement::IntoExisting);
    let verb = if adds { "add" } else { "create" };
    if project.dry_run {
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
        // A template from outside says what it will run as well as what it
        // will write: its commands are the part a tree does not show.
        if template.origin != Origin::BuiltIn && !project.no_hooks && !planned.hooks.is_empty() {
            let commands: Vec<String> = planned.hooks.iter().map(|h| h.run.join(" ")).collect();
            println!("Then it runs: {}\n", commands.join(" · "));
        }
        if !confirm(&format!("{} them?", capitalise(verb)))? {
            println!("Nothing was written.");
            return Ok(());
        }
    }

    // Asked before the files land: `git init` is one of the hooks, and after
    // it every destination would look like a repository.
    let in_a_repository = inside_a_repository(root);

    generate::write(plan, root)?;
    let done = if adds { "Added" } else { "Created" };
    println!("{done} {} in `{}`.", plural(plan.files.len()), root.display());

    if !project.no_hooks {
        run_hooks(&planned.hooks, root, in_a_repository)?;
    }

    let uncommitted = in_a_repository && planned.hooks.iter().any(|h| h.starts_repository);
    match planned.form {
        Some(form) => print_next_steps(name, form, project, root, uncommitted),
        None => print_cargo_next_steps(plan, root, uncommitted),
    }
    Ok(())
}

/// The flags that describe a project of the line mean nothing to a
/// cargo-generate template, and would be accepted and ignored.
fn refuse_what_cargo_generate_ignores(project: &ProjectArgs) -> Result<(), Box<dyn Error>> {
    let identity = &project.identity;
    let given = [
        ("--accent", identity.accent.is_some()),
        ("--description", identity.description.is_some()),
        ("--repo", identity.repo.is_some()),
        ("--host", project.host.is_some()),
        ("--with", !project.with.is_empty()),
    ];
    let named: Vec<&str> = given.iter().filter(|(_, is)| *is).map(|(flag, _)| *flag).collect();
    if named.is_empty() {
        return Ok(());
    }
    Err(format!(
        "{} {} nothing to a cargo-generate template, which asks with its own placeholders - answer them with `--define key=value`",
        named.join(", "),
        if named.len() == 1 { "means" } else { "mean" }
    )
    .into())
}

/// `--define key=value`, each split once at the first `=`.
fn parse_defines(defines: &[String]) -> Result<Vec<(String, String)>, Box<dyn Error>> {
    defines
        .iter()
        .map(|d| match d.split_once('=') {
            Some((key, value)) if !key.trim().is_empty() => Ok((key.trim().to_string(), value.to_string())),
            _ => Err(format!("`--define {d}` is not `key=value`").into()),
        })
        .collect()
}

/// A refusal to overwrite, with the way round it for a project being started
/// in place: most often the files in the way are the README, LICENSE and
/// .gitignore a hosting service puts in a new repository.
fn with_the_other_way(error: GenerateError, form: Option<Form>) -> GenerateError {
    match error {
        GenerateError::WouldOverwrite { root, paths, .. } if form != Some(Form::Docs) => {
            let one = paths.len() == 1;
            let hint = match (form.is_some(), one) {
                (true, true) => "move it aside and run again, or `lyrn adopt` to add only the standard files that are missing",
                (true, false) => "move them aside and run again, or `lyrn adopt` to add only the standard files that are missing",
                (false, true) => "move it aside and run again",
                (false, false) => "move them aside and run again",
            };
            GenerateError::WouldOverwrite { root, paths, hint: Some(hint) }
        }
        other => other,
    }
}

/// Whether `dir` is inside a git work tree already - its own repository, or
/// one it is a directory of.
pub fn inside_a_repository(dir: &Path) -> bool {
    // A directory that does not exist yet is in whatever its nearest existing
    // parent is in: `lyrn new` inside a checkout creates a directory of it.
    let existing = dir.ancestors().find(|a| a.as_os_str().is_empty() || a.is_dir());
    let at = match existing {
        Some(a) if !a.as_os_str().is_empty() => a,
        _ => Path::new("."),
    };
    Process::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(at)
        .output()
        .is_ok_and(|out| out.status.success() && String::from_utf8_lossy(&out.stdout).trim() == "true")
}

fn run_hooks(hooks: &[Hook], root: &Path, in_a_repository: bool) -> Result<(), Box<dyn Error>> {
    for hook in hooks {
        // A repository that already exists belongs to someone: its history,
        // its staging area and the moment of its next commit are theirs, and
        // a nested `git init` inside it would split it in two.
        if hook.starts_repository && in_a_repository {
            continue;
        }
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

fn print_next_steps(name: &str, form: Form, project: &ProjectArgs, root: &Path, uncommitted: bool) {
    println!("\nNext:");
    // The site is a project inside the repository; its commands run there.
    let workdir = if form == Form::Docs { root.join("docs") } else { root.to_path_buf() };
    // `Path::join` keeps a leading `./`, which reads as noise here.
    let workdir = workdir.strip_prefix(".").map(Path::to_path_buf).unwrap_or(workdir);
    if !workdir.as_os_str().is_empty() {
        println!("  cd {}", workdir.display().to_string().replace('\\', "/"));
    }
    if uncommitted {
        // Started inside a repository lyrn did not start, so nothing was
        // staged or committed - the first commit is the owner's to make.
        println!("  git add . && git commit -m \"feat: scaffold the project with lyrn\"");
    }
    let args = project;
    match form {
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
            if args.with.iter().any(|a| a == "stand") {
                println!("  pnpm stand");
            }
        }
        Form::Workspace => {
            // Named, because a workspace has more than one binary target the
            // moment anyone adds a second crate, and `cargo run` then refuses.
            println!("  cargo run --package {name} -- hello");
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

/// What to do next with a project from a cargo-generate template: go there,
/// and build it if it is a Cargo project - which is all lyrn knows about it.
fn print_cargo_next_steps(plan: &Plan, root: &Path, uncommitted: bool) {
    println!("\nNext:");
    let workdir = root.strip_prefix(".").map(Path::to_path_buf).unwrap_or_else(|_| root.to_path_buf());
    if !workdir.as_os_str().is_empty() {
        println!("  cd {}", workdir.display().to_string().replace('\\', "/"));
    }
    if uncommitted {
        println!("  git add . && git commit -m \"feat: scaffold the project with lyrn\"");
    }
    if plan.files.iter().any(|f| f.path == Path::new("Cargo.toml")) {
        println!("  cargo build");
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
pub fn repo(name: &str, explicit: Option<&str>) -> String {
    if let Some(explicit) = explicit {
        return explicit.to_string();
    }
    let owner = git_config("github.user").or_else(gh_login).unwrap_or_else(|| "OWNER".to_string());
    format!("{owner}/{name}")
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

pub fn git_config(key: &str) -> Option<String> {
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
    use crate::model::TemplateManifest;

    fn git(dir: &Path, args: &[&str]) {
        let status = Process::new("git").args(args).current_dir(dir).output().unwrap().status;
        assert!(status.success(), "git {args:?} failed in {}", dir.display());
    }

    fn git_init_hook() -> Vec<Hook> {
        vec![Hook {
            name: "Starting the repository".to_string(),
            run: vec!["git".to_string(), "init".to_string(), "--initial-branch=main".to_string()],
            optional: false,
            dir: None,
            starts_repository: true,
        }]
    }

    #[test]
    fn a_directory_of_a_checkout_is_inside_a_repository() {
        let outer = tempfile::tempdir().unwrap();
        assert!(!inside_a_repository(outer.path()));
        git(outer.path(), &["init", "--quiet"]);
        std::fs::create_dir(outer.path().join("sub")).unwrap();
        assert!(inside_a_repository(&outer.path().join("sub")));
        // Not created yet: judged by the nearest directory that exists.
        assert!(inside_a_repository(&outer.path().join("sub").join("new-project")));
    }

    /// `lyrn new` or `init` inside somebody's checkout must not start a
    /// second repository inside the first.
    #[test]
    fn a_repository_that_exists_is_not_started_again() {
        let outer = tempfile::tempdir().unwrap();
        git(outer.path(), &["init", "--quiet"]);
        let project = outer.path().join("demo-app");
        std::fs::create_dir(&project).unwrap();

        run_hooks(&git_init_hook(), &project, true).unwrap();
        assert!(!project.join(".git").exists(), "a nested repository was started");

        run_hooks(&git_init_hook(), &project, false).unwrap();
        assert!(project.join(".git").exists(), "outside a repository the hook must run");
    }

    /// What a test asks for: a name and an identity, the rest fixed.
    struct Args {
        name: String,
        identity: IdentityArgs,
    }

    impl Args {
        fn wanted(&self) -> Wanted<'_> {
            Wanted {
                name: &self.name,
                form: Form::Spa,
                host: None,
                with: &[],
                identity: &self.identity,
            }
        }
    }

    /// The built-in spa form, with its manifest swapped for `manifest`.
    fn spa_with(manifest: TemplateManifest) -> Native {
        let Kind::Lyrn(mut native) = template::builtin(Form::Spa).kind else {
            unreachable!()
        };
        native.manifest = manifest;
        native
    }

    fn context(args: &Args, manifest: TemplateManifest) -> Result<Context, Box<dyn Error>> {
        build_context(&args.wanted(), &spa_with(manifest), &Origin::BuiltIn, false)
    }

    fn args(name: &str) -> Args {
        Args {
            name: name.to_string(),
            identity: IdentityArgs {
                author: Some("Tester".to_string()),
                repo: Some("tester/demo-app".to_string()),
                ..IdentityArgs::default()
            },
        }
    }

    #[test]
    fn a_non_interactive_run_needs_no_answers() {
        let manifest = TemplateManifest::default();
        let context = context(&args("demo-app"), manifest.clone()).unwrap();
        assert_eq!(context.get("name"), Some("demo-app"));
        assert_eq!(context.get("title"), Some("Demo App"));
        assert_eq!(context.get("accent"), Some(PLACEHOLDER_ACCENT));
    }

    #[test]
    fn a_bad_name_is_refused_before_anything_is_written() {
        let manifest = TemplateManifest::default();
        assert!(context(&args("Demo App"), manifest.clone()).is_err());
    }

    #[test]
    fn an_accent_names_a_product_of_the_line() {
        let manifest = TemplateManifest::default();
        let mut a = args("demo-app");
        a.identity.accent = Some("kilna".to_string());
        let context = context(&a, manifest.clone()).unwrap();
        assert_eq!(context.get("accent"), Some("#D9569E"));
    }

    /// A project generated without an accent still wears the umbrella mark,
    /// and `lyrn.toml` says so. The doctor reads this rather than comparing
    /// the icon against a copy of the placeholder it would have to carry.
    #[test]
    fn a_project_without_an_accent_is_recorded_as_unmarked() {
        let manifest = TemplateManifest::default();
        let context = context(&args("demo-app"), manifest.clone()).unwrap();
        assert_eq!(context.get("mark"), Some("placeholder"));
    }

    /// Naming a colour is what choosing a mark looks like from here: the
    /// accent is the one thing `lyrn new` learns about a product's identity,
    /// and dowel derives the rest of the palette from it.
    #[test]
    fn choosing_an_accent_is_choosing_a_mark() {
        let manifest = TemplateManifest::default();
        let mut a = args("demo-app");
        a.identity.accent = Some("kilna".to_string());
        assert_eq!(context(&a, manifest.clone()).unwrap().get("mark"), Some("chosen"));

        // A literal colour counts too - a product may have a mark before it
        // has a place in the line's registry.
        let mut b = args("demo-app");
        b.identity.accent = Some("#123456".to_string());
        assert_eq!(context(&b, manifest.clone()).unwrap().get("mark"), Some("chosen"));
    }

    /// The placeholder's own hex, given explicitly, is still the placeholder.
    /// Otherwise `--accent '#6E7079'` would silently promote an unmarked
    /// project to a marked one, and the doctor would stop asking.
    #[test]
    fn spelling_out_the_placeholder_colour_is_not_choosing_a_mark() {
        let manifest = TemplateManifest::default();
        let mut a = args("demo-app");
        a.identity.accent = Some(PLACEHOLDER_ACCENT.to_string());
        assert_eq!(context(&a, manifest.clone()).unwrap().get("mark"), Some("placeholder"));
    }

    #[test]
    fn an_unknown_accent_is_refused() {
        let manifest = TemplateManifest::default();
        let mut a = args("demo-app");
        a.identity.accent = Some("chartreuse".to_string());
        assert!(context(&a, manifest.clone()).is_err());
    }

    #[test]
    fn the_standard_comes_from_the_manifest() {
        let manifest = TemplateManifest {
            standard: "2026.09".to_string(),
            ..Default::default()
        };
        let context = context(&args("demo-app"), manifest.clone()).unwrap();
        assert_eq!(context.get("standard"), Some("2026.09"));
    }
}
