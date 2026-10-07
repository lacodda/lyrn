//! Templates written for cargo-generate, read the way cargo-generate reads
//! them.
//!
//! The files are Liquid, rendered by the same Liquid engine cargo-generate
//! uses, with its variables (`project-name`, `crate_name`, `crate_type`,
//! `authors`, `username`, `os-arch`, `is_init`, `within_cargo_project`) and
//! its case filters, computed by the same crate. `cargo-generate.toml` is
//! honoured for what lyrn can do honestly - `ignore`, `include`, `exclude`,
//! `[placeholders]`, `[conditional]`, `vcs`, `.genignore`, the `.liquid`
//! suffix - and refused for what it cannot: Rhai hooks, the `rhai` filter,
//! sub-templates. A template run without its hooks would produce a different
//! project, and nothing would say so.
//!
//! As in cargo-generate, a file whose Liquid does not render is kept as it
//! is, with a warning: templates lean on that for files that happen to
//! contain braces.

use std::error::Error;
use std::fmt;
use std::path::Path;

use heck::{ToKebabCase, ToLowerCamelCase, ToPascalCase, ToShoutyKebabCase, ToShoutySnakeCase, ToSnakeCase, ToTitleCase, ToUpperCamelCase};
use liquid_core::parser::{FilterArguments, ParameterReflection};
use liquid_core::{Filter, FilterReflection, ParseFilter, Runtime, Value, ValueView};
use serde::Deserialize;

use super::glob::Patterns;
use super::{DiskFile, body_of, read_tree};
use crate::generate::{self, Body, GenerateError, Plan, PlannedFile};
use crate::model::Hook;

const CONFIG: &str = "cargo-generate.toml";
const GENIGNORE: &str = ".genignore";

/// Whether a directory is a cargo-generate template: it says so with
/// `cargo-generate.toml`, or it is a Rust project with placeholders in it and
/// no manifest at all, the way older templates look.
pub fn is_one(dir: &Path) -> bool {
    dir.join(CONFIG).is_file() || dir.join("Cargo.toml").is_file()
}

/// A cargo-generate template, read.
pub struct CargoTemplate {
    pub config: Config,
    /// Every file but the template's own machinery, before any `ignore` -
    /// which can depend on the answers - is applied.
    files: Vec<SourceFile>,
    /// The lines of `.genignore`.
    genignore: Vec<String>,
}

struct SourceFile {
    path: String,
    body: Body,
    executable: bool,
}

/// `cargo-generate.toml`.
#[derive(Debug, Default, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub template: Section,
    #[serde(default)]
    pub placeholders: toml::Table,
    #[serde(default)]
    pub conditional: toml::Table,
    #[serde(default)]
    pub hooks: Option<toml::Table>,
}

/// `[template]`. Fields cargo-generate knows and lyrn has no use for - the
/// cargo-generate version, `force`, a description - are read past.
#[derive(Debug, Default, Deserialize)]
pub struct Section {
    #[serde(default)]
    pub ignore: Vec<String>,
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default)]
    pub sub_templates: Vec<String>,
    #[serde(default)]
    pub vcs: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Placeholder {
    #[serde(rename = "type")]
    kind: String,
    prompt: String,
    #[serde(default)]
    choices: Option<Vec<String>>,
    #[serde(default)]
    default: Option<toml::Value>,
    #[serde(default)]
    regex: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct Conditional {
    #[serde(default)]
    ignore: Vec<String>,
    #[serde(default)]
    include: Vec<String>,
    #[serde(default)]
    exclude: Vec<String>,
    #[serde(default)]
    placeholders: toml::Table,
}

/// Read a cargo-generate template, refusing what lyrn cannot run.
pub fn load(dir: &Path) -> Result<CargoTemplate, Box<dyn Error>> {
    let config: Config = match std::fs::read_to_string(dir.join(CONFIG)) {
        Ok(text) => toml::from_str(&text).map_err(|e| format!("`{}`: {e}", dir.join(CONFIG).display()))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Config::default(),
        Err(e) => return Err(e.into()),
    };

    let hooks: Vec<String> = config
        .hooks
        .iter()
        .flat_map(|table| table.iter())
        .filter(|(_, scripts)| scripts.as_array().is_none_or(|a| !a.is_empty()))
        .map(|(stage, _)| stage.clone())
        .collect();
    if !hooks.is_empty() {
        return Err(format!(
            "the template runs Rhai hooks ({}), which lyrn does not run; without them it would generate a different project than cargo-generate does - use `cargo generate` for it",
            hooks.join(", ")
        )
        .into());
    }
    if !config.template.sub_templates.is_empty() {
        return Err(format!(
            "the template holds sub-templates ({}); name the one you want by its directory: `--template <path>/<sub-template>`",
            config.template.sub_templates.join(", ")
        )
        .into());
    }
    if !config.template.include.is_empty() && !config.template.exclude.is_empty() {
        return Err("`cargo-generate.toml` sets both `include` and `exclude`; cargo-generate takes one or the other".into());
    }
    // Checked now rather than on first use, so a broken pattern is a broken
    // template rather than a surprise halfway through generating.
    for patterns in [&config.template.ignore, &config.template.include, &config.template.exclude] {
        Patterns::new(patterns)?;
    }

    let genignore = match std::fs::read_to_string(dir.join(GENIGNORE)) {
        Ok(text) => text.lines().map(str::to_string).collect(),
        Err(_) => Vec::new(),
    };
    Patterns::new(&genignore).map_err(|e| format!("`{GENIGNORE}`: {e}"))?;

    let files = read_tree(dir, &[".git"])?
        .into_iter()
        .filter(|f| f.path != CONFIG && f.path != GENIGNORE)
        .map(|DiskFile { path, bytes, executable }| SourceFile {
            path,
            body: body_of(bytes),
            executable,
        })
        .collect();

    Ok(CargoTemplate { config, files, genignore })
}

/// What cargo-generate learns about the machine and the destination rather
/// than asking.
pub struct Facts {
    /// `Name <email>`, from git.
    pub authors: String,
    pub username: String,
    /// `lyrn init`, as cargo-generate's `--init`.
    pub is_init: bool,
    /// The destination is inside a Cargo project already.
    pub within_cargo_project: bool,
}

/// Where the answers to placeholders come from.
pub struct Answers<'a> {
    /// `--define key=value`.
    pub defines: &'a [(String, String)],
    /// Whether someone is at the terminal to be asked.
    pub interactive: bool,
}

impl CargoTemplate {
    /// The hooks lyrn runs for this template: starting the repository, as
    /// cargo-generate does, unless the template says `vcs = "none"`.
    pub fn hooks(&self) -> Vec<Hook> {
        if self.config.template.vcs.as_deref().is_some_and(|v| v.eq_ignore_ascii_case("none")) {
            return Vec::new();
        }
        let hook = |name: &str, run: &[&str]| Hook {
            name: name.to_string(),
            run: run.iter().map(|s| s.to_string()).collect(),
            optional: true,
            dir: None,
            starts_repository: true,
        };
        vec![
            hook("Starting the repository", &["git", "init", "--initial-branch=main"]),
            hook("Staging the first commit", &["git", "add", "."]),
            hook("Creating the first commit", &["git", "commit", "-m", "feat: scaffold the project with lyrn"]),
        ]
    }

    /// Answer the placeholders and render every file into a plan.
    pub fn plan(&self, name: &str, facts: &Facts, answers: &Answers) -> Result<Plan, Box<dyn Error>> {
        let parser = parser()?;
        let mut globals = liquid::Object::new();
        let set = |globals: &mut liquid::Object, key: &str, value: Value| {
            globals.insert(key.to_string().into(), value);
        };
        set(&mut globals, "project-name", Value::scalar(name.to_string()));
        set(&mut globals, "crate_name", Value::scalar(name.to_snake_case()));
        set(&mut globals, "crate_type", Value::scalar("bin"));
        set(&mut globals, "authors", Value::scalar(facts.authors.clone()));
        set(&mut globals, "username", Value::scalar(facts.username.clone()));
        set(
            &mut globals,
            "os-arch",
            Value::scalar(format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)),
        );
        set(&mut globals, "is_init", Value::scalar(facts.is_init));
        set(&mut globals, "within_cargo_project", Value::scalar(facts.within_cargo_project));

        // Every name a `--define` may set: the template's placeholders, its
        // conditional ones, and the one built-in cargo-generate lets a flag
        // choose. A define naming nothing is a typo, and would be silently
        // unused.
        let mut askable: Vec<String> = vec!["crate_type".to_string()];
        askable.extend(self.config.placeholders.keys().cloned());
        for (_, conditional) in &self.config.conditional {
            if let Some(table) = conditional.get("placeholders").and_then(|p| p.as_table()) {
                askable.extend(table.keys().cloned());
            }
        }
        for (key, _) in answers.defines {
            if !askable.contains(key) {
                return Err(format!(
                    "`--define {key}=...`: the template asks nothing called `{key}` (it asks: {})",
                    askable.join(", ")
                )
                .into());
            }
        }
        if let Some((_, value)) = answers.defines.iter().rev().find(|(k, _)| k == "crate_type") {
            set(&mut globals, "crate_type", Value::scalar(value.clone()));
        }

        ask_all(&self.config.placeholders, &mut globals, answers)?;

        let mut ignore = self.config.template.ignore.clone();
        ignore.extend(self.genignore.iter().cloned());
        let mut include = self.config.template.include.clone();
        let mut exclude = self.config.template.exclude.clone();
        for (condition, conditional) in &self.config.conditional {
            let conditional: Conditional = conditional
                .clone()
                .try_into()
                .map_err(|e| format!("`[conditional.'{condition}']` in {CONFIG}: {e}"))?;
            if holds(&parser, condition, &globals)? {
                ask_all(&conditional.placeholders, &mut globals, answers)?;
                ignore.extend(conditional.ignore);
                include.extend(conditional.include);
                exclude.extend(conditional.exclude);
            }
        }
        let ignore = Patterns::new(&ignore)?;
        let include = Patterns::new(&include)?;
        let exclude = Patterns::new(&exclude)?;

        let mut files = Vec::new();
        for file in &self.files {
            if ignore.matches(&file.path) {
                continue;
            }
            let rendered = render_gracefully(&parser, &file.path, &file.path, &globals);
            let rendered = rendered.strip_suffix(".liquid").unwrap_or(&rendered).to_string();
            let path = generate::inside_the_project(&rendered).ok_or_else(|| GenerateError::OutsideTheProject {
                file: file.path.clone(),
                rendered: rendered.clone(),
            })?;

            let render = if include.is_empty() {
                !exclude.matches(&file.path)
            } else {
                include.matches(&file.path)
            };
            let contents = match &file.body {
                Body::Text(text) if render => render_gracefully(&parser, &file.path, text, &globals).into_bytes(),
                Body::Text(text) => text.as_bytes().to_vec(),
                Body::Binary(bytes) => bytes.clone(),
            };
            files.push(PlannedFile {
                path,
                contents,
                executable: file.executable,
            });
        }
        files.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(Plan { files })
    }
}

/// Answer every placeholder of a table, in the order it declares them.
fn ask_all(table: &toml::Table, globals: &mut liquid::Object, answers: &Answers) -> Result<(), Box<dyn Error>> {
    for (key, spec) in table {
        let placeholder: Placeholder = spec.clone().try_into().map_err(|e| format!("the placeholder `{key}` in {CONFIG}: {e}"))?;
        let value = answer(key, &placeholder, answers)?;
        globals.insert(key.clone().into(), value);
    }
    Ok(())
}

/// One placeholder's answer: a `--define`, else a question, else its default.
fn answer(key: &str, placeholder: &Placeholder, answers: &Answers) -> Result<Value, Box<dyn Error>> {
    let defined = answers.defines.iter().rev().find(|(k, _)| k == key).map(|(_, v)| v.clone());
    let default = placeholder.default.as_ref().map(|d| match d {
        toml::Value::String(s) => s.clone(),
        other => other.to_string(),
    });

    match placeholder.kind.as_str() {
        "bool" => {
            let parse = |text: &str| match text {
                "true" => Ok(true),
                "false" => Ok(false),
                other => Err(format!("`{key}` is true or false, not `{other}`")),
            };
            let value = match (defined, answers.interactive) {
                (Some(text), _) => parse(&text)?,
                (None, true) => dialoguer::Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
                    .with_prompt(&placeholder.prompt)
                    .default(default.as_deref().map(parse).transpose()?.unwrap_or(false))
                    .interact()?,
                (None, false) => match default {
                    Some(text) => parse(&text)?,
                    None => return Err(missing(key, placeholder)),
                },
            };
            Ok(Value::scalar(value))
        }
        "string" => {
            let value = match (defined, answers.interactive, &placeholder.choices) {
                (Some(text), _, _) => text,
                (None, true, Some(choices)) => {
                    let start = default.as_ref().and_then(|d| choices.iter().position(|c| c == d)).unwrap_or(0);
                    let picked = dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
                        .with_prompt(&placeholder.prompt)
                        .items(choices)
                        .default(start)
                        .interact()?;
                    choices[picked].clone()
                }
                (None, true, None) => {
                    let theme = dialoguer::theme::ColorfulTheme::default();
                    let mut input = dialoguer::Input::<String>::with_theme(&theme).with_prompt(&placeholder.prompt);
                    if let Some(default) = &default {
                        input = input.default(default.clone());
                    }
                    input.interact_text()?
                }
                (None, false, _) => default.ok_or_else(|| missing(key, placeholder))?,
            };
            if let Some(choices) = &placeholder.choices
                && !choices.contains(&value)
            {
                return Err(format!("`{key}` is one of {}, not `{value}`", choices.join(", ")).into());
            }
            if let Some(pattern) = &placeholder.regex {
                let regex = regex::Regex::new(pattern).map_err(|e| format!("the placeholder `{key}` carries a regex that does not compile: {e}"))?;
                if !regex.is_match(&value) {
                    return Err(format!("`{value}` does not match what `{key}` asks for (`{pattern}`)").into());
                }
            }
            Ok(Value::scalar(value))
        }
        other => Err(format!("the placeholder `{key}` has the type `{other}`; cargo-generate knows `string` and `bool`").into()),
    }
}

fn missing(key: &str, placeholder: &Placeholder) -> Box<dyn Error> {
    format!(
        "the template asks \"{}\" and there is nobody at a terminal to answer; pass `--define {key}=<value>`",
        placeholder.prompt
    )
    .into()
}

/// Whether a `[conditional]` expression holds - read by Liquid itself, so the
/// expression means exactly what it means in cargo-generate.
fn holds(parser: &liquid::Parser, condition: &str, globals: &liquid::Object) -> Result<bool, Box<dyn Error>> {
    let template = parser
        .parse(&format!("{{% if {condition} %}}true{{% endif %}}"))
        .map_err(|e| format!("`[conditional.'{condition}']` is not an expression Liquid reads: {e}"))?;
    let rendered = template
        .render(globals)
        .map_err(|e| format!("`[conditional.'{condition}']` does not evaluate: {e}"))?;
    Ok(rendered == "true")
}

/// Render `text`, or keep it as it is with a warning if its Liquid does not
/// render - cargo-generate's own behaviour, which templates rely on.
fn render_gracefully(parser: &liquid::Parser, path: &str, text: &str, globals: &liquid::Object) -> String {
    if !text.contains("{{") && !text.contains("{%") {
        return text.to_string();
    }
    let outcome = parser.parse(text).and_then(|t| t.render(globals));
    match outcome {
        Ok(rendered) => rendered,
        Err(e) => {
            let reason = e.to_string();
            let first = reason.lines().find(|l| !l.trim().is_empty()).unwrap_or("it does not render");
            eprintln!("warning: `{path}` is kept as it is: {}", first.trim());
            text.to_string()
        }
    }
}

/// The Liquid cargo-generate speaks: the standard library plus its filters.
fn parser() -> Result<liquid::Parser, Box<dyn Error>> {
    let mut builder = liquid::ParserBuilder::with_stdlib();
    for filter in CASE_FILTERS {
        builder = builder.filter(filter.clone());
    }
    Ok(builder.filter(Rhai).build()?)
}

/// A filter that changes the case of its input, named as cargo-generate
/// names it.
#[derive(Clone)]
struct CaseFilter {
    name: &'static str,
    convert: fn(&str) -> String,
}

const CASE_FILTERS: &[CaseFilter] = &[
    CaseFilter {
        name: "kebab_case",
        convert: |s| s.to_kebab_case(),
    },
    CaseFilter {
        name: "lower_camel_case",
        convert: |s| s.to_lower_camel_case(),
    },
    CaseFilter {
        name: "pascal_case",
        convert: |s| s.to_pascal_case(),
    },
    CaseFilter {
        name: "shouty_kebab_case",
        convert: |s| s.to_shouty_kebab_case(),
    },
    CaseFilter {
        name: "shouty_snake_case",
        convert: |s| s.to_shouty_snake_case(),
    },
    CaseFilter {
        name: "snake_case",
        convert: |s| s.to_snake_case(),
    },
    CaseFilter {
        name: "title_case",
        convert: |s| s.to_title_case(),
    },
    CaseFilter {
        name: "upper_camel_case",
        convert: |s| s.to_upper_camel_case(),
    },
];

impl FilterReflection for CaseFilter {
    fn name(&self) -> &str {
        self.name
    }
    fn description(&self) -> &str {
        "Changes the case of the input, as cargo-generate does."
    }
    fn positional_parameters(&self) -> &'static [ParameterReflection] {
        &[]
    }
    fn keyword_parameters(&self) -> &'static [ParameterReflection] {
        &[]
    }
}

impl ParseFilter for CaseFilter {
    fn parse(&self, mut arguments: FilterArguments) -> liquid_core::Result<Box<dyn Filter>> {
        if arguments.positional.next().is_some() || arguments.keyword.next().is_some() {
            return Err(liquid_core::Error::with_msg(format!("`{}` takes no arguments", self.name)));
        }
        Ok(Box::new(AppliedCase {
            name: self.name,
            convert: self.convert,
        }))
    }
    fn reflection(&self) -> &dyn FilterReflection {
        self
    }
}

#[derive(Debug)]
struct AppliedCase {
    name: &'static str,
    convert: fn(&str) -> String,
}

impl fmt::Display for AppliedCase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name)
    }
}

impl Filter for AppliedCase {
    fn evaluate(&self, input: &dyn ValueView, _runtime: &dyn Runtime) -> liquid_core::Result<Value> {
        Ok(Value::scalar((self.convert)(input.to_kstr().as_str())))
    }
}

/// cargo-generate's `rhai` filter runs a script. lyrn does not run scripts, so
/// the filter exists only to say that, rather than as an unknown name.
#[derive(Clone)]
struct Rhai;

impl FilterReflection for Rhai {
    fn name(&self) -> &str {
        "rhai"
    }
    fn description(&self) -> &str {
        "Refused: lyrn does not run Rhai."
    }
    fn positional_parameters(&self) -> &'static [ParameterReflection] {
        &[]
    }
    fn keyword_parameters(&self) -> &'static [ParameterReflection] {
        &[]
    }
}

impl ParseFilter for Rhai {
    fn parse(&self, _arguments: FilterArguments) -> liquid_core::Result<Box<dyn Filter>> {
        Err(liquid_core::Error::with_msg("the `rhai` filter runs a Rhai script, which lyrn does not run"))
    }
    fn reflection(&self) -> &dyn FilterReflection {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn facts() -> Facts {
        Facts {
            authors: "Jane Doe <jane@example.com>".into(),
            username: "Jane Doe".into(),
            is_init: false,
            within_cargo_project: false,
        }
    }

    fn quiet(defines: &[(String, String)]) -> Answers<'_> {
        Answers { defines, interactive: false }
    }

    fn write(dir: &Path, path: &str, text: &str) {
        let target = dir.join(path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, text).unwrap();
    }

    fn file<'a>(plan: &'a Plan, path: &str) -> &'a str {
        let found = plan
            .files
            .iter()
            .find(|f| f.path == Path::new(path))
            .unwrap_or_else(|| panic!("no `{path}` in the plan"));
        std::str::from_utf8(&found.contents).unwrap()
    }

    fn has(plan: &Plan, path: &str) -> bool {
        plan.files.iter().any(|f| f.path == Path::new(path))
    }

    #[test]
    fn the_builtin_variables_and_filters_render() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "Cargo.toml",
            "name = \"{{project-name}}\"\nauthors = [\"{{authors}}\"]\n# {{ crate_name }} {{ crate_type }} {{ is_init }}\n",
        );
        write(
            dir.path(),
            "src/main.rs",
            "struct {{ project-name | pascal_case }};\nconst N: &str = \"{{ project-name | shouty_snake_case }}\";\n",
        );
        let plan = load(dir.path()).unwrap().plan("word-count", &facts(), &quiet(&[])).unwrap();
        assert_eq!(
            file(&plan, "Cargo.toml"),
            "name = \"word-count\"\nauthors = [\"Jane Doe <jane@example.com>\"]\n# word_count bin false\n"
        );
        assert_eq!(file(&plan, "src/main.rs"), "struct WordCount;\nconst N: &str = \"WORD_COUNT\";\n");
    }

    #[test]
    fn names_render_and_lose_the_liquid_suffix() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "Cargo.toml", "x");
        write(dir.path(), "src/{{crate_name}}.rs.liquid", "// {{ project-name }}\n");
        let plan = load(dir.path()).unwrap().plan("my-tool", &facts(), &quiet(&[])).unwrap();
        assert_eq!(file(&plan, "src/my_tool.rs"), "// my-tool\n");
    }

    #[test]
    fn the_template_machinery_is_not_copied() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "Cargo.toml", "x");
        write(dir.path(), CONFIG, "[template]\n");
        write(dir.path(), GENIGNORE, "notes.md\n");
        write(dir.path(), "notes.md", "x");
        write(dir.path(), ".git/HEAD", "ref: refs/heads/main\n");
        let plan = load(dir.path()).unwrap().plan("demo", &facts(), &quiet(&[])).unwrap();
        let paths: Vec<_> = plan.files.iter().map(|f| f.path.to_string_lossy().replace('\\', "/")).collect();
        assert_eq!(paths, vec!["Cargo.toml"]);
    }

    #[test]
    fn ignore_exclude_and_include_mean_what_cargo_generate_means() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "Cargo.toml", "{{ project-name }}");
        write(dir.path(), "raw.txt", "{{ project-name }}");
        write(dir.path(), "gone/file.txt", "x");
        write(dir.path(), CONFIG, "[template]\nignore = [\"gone/\"]\nexclude = [\"raw.txt\"]\n");
        let plan = load(dir.path()).unwrap().plan("demo", &facts(), &quiet(&[])).unwrap();
        assert_eq!(file(&plan, "Cargo.toml"), "demo");
        assert_eq!(file(&plan, "raw.txt"), "{{ project-name }}");
        assert!(!has(&plan, "gone/file.txt"));

        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "Cargo.toml", "{{ project-name }}");
        write(dir.path(), "raw.txt", "{{ project-name }}");
        write(dir.path(), CONFIG, "[template]\ninclude = [\"Cargo.toml\"]\n");
        let plan = load(dir.path()).unwrap().plan("demo", &facts(), &quiet(&[])).unwrap();
        assert_eq!(file(&plan, "Cargo.toml"), "demo");
        assert_eq!(file(&plan, "raw.txt"), "{{ project-name }}");
    }

    #[test]
    fn placeholders_come_from_defines_else_defaults() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "Cargo.toml", "{{ edition }} {{ async }}");
        write(
            dir.path(),
            CONFIG,
            "[placeholders.edition]\ntype = \"string\"\nprompt = \"Edition?\"\nchoices = [\"2021\", \"2024\"]\ndefault = \"2024\"\n\n[placeholders.async]\ntype = \"bool\"\nprompt = \"Async?\"\ndefault = false\n",
        );
        let template = load(dir.path()).unwrap();
        assert_eq!(file(&template.plan("demo", &facts(), &quiet(&[])).unwrap(), "Cargo.toml"), "2024 false");
        let defines = [("edition".to_string(), "2021".to_string()), ("async".to_string(), "true".to_string())];
        assert_eq!(file(&template.plan("demo", &facts(), &quiet(&defines)).unwrap(), "Cargo.toml"), "2021 true");
    }

    #[test]
    fn a_wrong_answer_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "Cargo.toml", "{{ edition }} {{ port }}");
        write(
            dir.path(),
            CONFIG,
            "[placeholders.edition]\ntype = \"string\"\nprompt = \"Edition?\"\nchoices = [\"2021\", \"2024\"]\ndefault = \"2024\"\n\n[placeholders.port]\ntype = \"string\"\nprompt = \"Port?\"\nregex = \"^[0-9]+$\"\ndefault = \"8080\"\n",
        );
        let template = load(dir.path()).unwrap();
        let not_a_choice = [("edition".to_string(), "2018".to_string())];
        assert!(template.plan("demo", &facts(), &quiet(&not_a_choice)).is_err());
        let not_a_number = [("port".to_string(), "eighty".to_string())];
        assert!(template.plan("demo", &facts(), &quiet(&not_a_number)).is_err());
        let a_typo = [("editon".to_string(), "2021".to_string())];
        let err = template.plan("demo", &facts(), &quiet(&a_typo)).unwrap_err().to_string();
        assert!(err.contains("asks nothing called `editon`"), "{err}");
    }

    #[test]
    fn a_placeholder_without_a_default_needs_an_answer() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "Cargo.toml", "{{ licence }}");
        write(dir.path(), CONFIG, "[placeholders.licence]\ntype = \"string\"\nprompt = \"Licence?\"\n");
        let err = load(dir.path()).unwrap().plan("demo", &facts(), &quiet(&[])).unwrap_err().to_string();
        assert!(err.contains("--define licence="), "{err}");
    }

    #[test]
    fn a_conditional_applies_when_its_expression_holds() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "Cargo.toml", "x");
        write(dir.path(), "src/main.rs", "fn main() {}");
        write(dir.path(), "src/lib.rs", "pub fn f() {}");
        write(
            dir.path(),
            CONFIG,
            "[conditional.'crate_type == \"lib\"']\nignore = [\"src/main.rs\"]\n\n[conditional.'crate_type == \"bin\"']\nignore = [\"src/lib.rs\"]\n",
        );
        let template = load(dir.path()).unwrap();
        let bin = template.plan("demo", &facts(), &quiet(&[])).unwrap();
        assert!(has(&bin, "src/main.rs") && !has(&bin, "src/lib.rs"));
        let lib = template
            .plan("demo", &facts(), &quiet(&[("crate_type".to_string(), "lib".to_string())]))
            .unwrap();
        assert!(!has(&lib, "src/main.rs") && has(&lib, "src/lib.rs"));
    }

    /// cargo-generate keeps a file whose Liquid does not render; templates
    /// rely on it for workflows full of `${{ }}`.
    #[test]
    fn a_file_that_does_not_render_is_kept_as_it_is() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "Cargo.toml", "x");
        write(dir.path(), "ci.yml", "runs-on: ${{ matrix.os }}\n");
        let plan = load(dir.path()).unwrap().plan("demo", &facts(), &quiet(&[])).unwrap();
        assert_eq!(file(&plan, "ci.yml"), "runs-on: ${{ matrix.os }}\n");
    }

    #[test]
    fn rhai_hooks_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "Cargo.toml", "x");
        write(dir.path(), CONFIG, "[hooks]\npre = [\"setup.rhai\"]\n");
        let err = load(dir.path()).err().unwrap().to_string();
        assert!(err.contains("Rhai hooks (pre)"), "{err}");
    }

    #[test]
    fn the_rhai_filter_is_refused_by_name() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "Cargo.toml", "{{ \"1 + 1\" | rhai }}");
        // Kept as it is with a warning, as any file whose Liquid fails.
        let plan = load(dir.path()).unwrap().plan("demo", &facts(), &quiet(&[])).unwrap();
        assert_eq!(file(&plan, "Cargo.toml"), "{{ \"1 + 1\" | rhai }}");
    }

    #[test]
    fn vcs_none_starts_no_repository() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "Cargo.toml", "x");
        assert_eq!(load(dir.path()).unwrap().hooks().len(), 3);
        write(dir.path(), CONFIG, "[template]\nvcs = \"none\"\n");
        assert!(load(dir.path()).unwrap().hooks().is_empty());
    }

    #[test]
    fn a_hostile_answer_cannot_write_outside() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "Cargo.toml", "x");
        write(dir.path(), "{{ where }}/x.txt", "x");
        write(dir.path(), CONFIG, "[placeholders.where]\ntype = \"string\"\nprompt = \"Where?\"\n");
        let defines = [("where".to_string(), "../..".to_string())];
        let err = load(dir.path()).unwrap().plan("demo", &facts(), &quiet(&defines)).unwrap_err().to_string();
        assert!(err.contains("outside the project"), "{err}");
    }
}
