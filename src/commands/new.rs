use std::error::Error;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::Command as Process;

use crate::cli::NewArgs;
use crate::generate;
use crate::model::{Context, Form, TemplateManifest};
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

    let accent = match &args.accent {
        Some(value) => naming::resolve_accent(value)?,
        None if interactive => prompt_accent()?,
        None => PLACEHOLDER_ACCENT.to_string(),
    };

    let description = match &args.description {
        Some(value) => value.clone(),
        None if interactive => prompt_line("What is it, in one line?", &format!("A {} on the lacodda line's stack.", args.form))?,
        None => format!("A {} on the lacodda line's stack.", args.form),
    };

    let author = match &args.author {
        Some(value) => value.clone(),
        None => git_config("user.name").unwrap_or_else(|| "The author".to_string()),
    };

    // Resolved once: the lookup can spawn `gh`, and it is asked for twice.
    let repo = repo(args);
    let owner = repo.split('/').next().unwrap_or("OWNER").to_string();

    let mut context = Context::new();
    context
        .set("name", &args.name)
        .set("title", naming::title_from_name(&args.name))
        .set("description", description)
        .set("accent", accent)
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

    Ok(context)
}

pub fn run(args: NewArgs) -> Result<(), Box<dyn Error>> {
    let manifest: TemplateManifest = toml::from_str(templates::manifest_for(args.form))?;
    let interactive = !args.assume_yes && std::io::stdin().is_terminal();

    let context = build_context(&args, &manifest, interactive)?;
    let root = args.path.clone().unwrap_or_else(|| PathBuf::from(&args.name));

    let plan = generate::plan_with(&templates::sources_for(args.form), &manifest, &context, &args.with)?;

    if args.dry_run {
        println!("Would create {} in `{}`:\n", plural(plan.files.len()), root.display());
        println!("{}", indent(&plan.tree()));
        return Ok(());
    }

    generate::check_destination(&root)?;
    generate::write(&plan, &root)?;
    println!("Created {} in `{}`.", plural(plan.files.len()), root.display());

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
        let outcome = Process::new(program).args(rest).current_dir(root).output();
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
    println!("  cd {}", root.display());
    match args.form {
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
