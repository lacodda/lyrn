//! `lyrn assets`: the mark laid out by the line's canon, and every raster
//! drawn from it.
//!
//! A product's mark is four masters: L, M and S, and the banner. Everything
//! else a repository carries of it - the `.ico` its build embeds, the favicon,
//! the touch icon, the site's copies, the social preview - is drawn from them
//! and never edited by hand. Every product of the line once drew those with an
//! exporter of its own, and nine of fifteen put a wrong level somewhere: a
//! whole `.ico` cut from the S tile, a touch icon from S at 180px. One drawing,
//! here, by the rule, is the end of that.

use std::collections::BTreeMap;
use std::error::Error;
use std::path::{Path, PathBuf};

use crate::cli::AssetsArgs;
use crate::commands::check::product::Product;
use crate::commands::doctor::{Repo, is_test_material};
use crate::mark::{Level, Mark, Masters};
use crate::model::Form;

/// Where the masters live in every repository of the line.
pub const L: &str = "assets/logo.svg";
pub const M: &str = "assets/logo-m.svg";
pub const S: &str = "assets/logo-s.svg";
pub const BANNER: &str = "assets/banner.svg";
pub const SOCIAL: &str = "assets/social-preview.png";

/// The umbrella mark's name among the masters: a project wearing it has not
/// chosen a mark of its own.
const UMBRELLA: &str = "lacodda";

/// One file `lyrn assets` writes, and what it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    pub path: String,
    pub bytes: Vec<u8>,
    pub what: String,
}

pub fn run(args: AssetsArgs) -> Result<(), Box<dyn Error>> {
    let path = args.path.unwrap_or_else(|| PathBuf::from("."));
    if !path.is_dir() {
        return Err(format!("`{}` is not a directory - `lyrn assets` draws a repository's mark", path.display()).into());
    }
    let repo = Repo::read(&path);
    let facts = repo.facts.as_ref().map_err(|e| e.clone())?;
    let name = facts
        .name
        .clone()
        .or_else(|| repo.root.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_default();
    println!("lyrn assets: {}, {name}\n", repo.root.display());

    let mut written: Vec<(String, String)> = Vec::new();
    let mut unchanged: Vec<(String, String)> = Vec::new();
    let mut put = |output: Output| -> Result<(), Box<dyn Error>> {
        let file = repo.root.join(&output.path);
        if std::fs::read(&file).is_ok_and(|bytes| bytes == output.bytes) {
            unchanged.push((output.path, output.what));
            return Ok(());
        }
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&file, &output.bytes).map_err(|e| format!("{}: {e}", output.path))?;
        written.push((output.path, output.what));
        Ok(())
    };

    // The masters, from where they were drawn, when asked to take them in.
    let mut chosen_now = false;
    if let Some(from) = &args.from {
        let mark = args.mark.clone().unwrap_or_else(|| name.clone());
        for output in take_masters(from, &mark)? {
            put(output)?;
        }
        chosen_now = mark != UMBRELLA && mark_the_choice(&repo.root)?;
    }

    let mark = read_mark(&repo.root)?;
    let form = facts.form(&repo.root).ok().map(|(form, _)| form);
    for output in outputs(&repo, &mark, form, &name) {
        put(output)?;
    }

    let width = written.iter().chain(&unchanged).map(|(p, _)| p.len()).max().unwrap_or(0);
    for (path, what) in &written {
        println!("wrote      {path:width$}  {what}");
    }
    for (path, what) in &unchanged {
        println!("unchanged  {path:width$}  {what}");
    }
    if chosen_now {
        println!("\nlyrn.toml: `mark = \"chosen\"` - the project wears a mark of its own now");
    }
    if written.iter().any(|(p, _)| p == SOCIAL) {
        let settings = Product::read(&repo)
            .github
            .map(|r| format!("https://github.com/{r}/settings"))
            .unwrap_or_else(|| "the repository's settings on GitHub".to_string());
        println!("\n{SOCIAL} changed: upload it by hand at {settings} → Social preview - GitHub has no API for it");
    }
    for script in repo.files.iter().filter(|f| f.ends_with("export-assets.mjs") && !is_test_material(f)) {
        println!("\n{script} draws these too, by a rule of its own: delete it - `lyrn assets` is the one drawing");
    }
    Ok(())
}

/// The four masters a mark's generator writes - `<mark>-L.svg`, `-M`, `-S`
/// and `-banner` - as the files the canon keeps them in. A mark with no
/// banner, as the umbrella mark has none, is taken without one.
fn take_masters(from: &Path, mark: &str) -> Result<Vec<Output>, Box<dyn Error>> {
    let read = |suffix: &str| std::fs::read(from.join(format!("{mark}-{suffix}.svg"))).ok();
    let (Some(l), Some(m), Some(s)) = (read("L"), read("M"), read("S")) else {
        let mut marks: Vec<String> = std::fs::read_dir(from)
            .map_err(|e| format!("{}: {e}", from.display()))?
            .filter_map(Result::ok)
            .filter_map(|e| e.file_name().to_string_lossy().strip_suffix("-L.svg").map(str::to_string))
            .collect();
        marks.sort();
        let known = if marks.is_empty() {
            "it holds no `<mark>-L.svg` at all".to_string()
        } else {
            format!("the marks there: {} - `--mark` names one", marks.join(", "))
        };
        return Err(format!("{} has no {mark}-L.svg, {mark}-M.svg and {mark}-S.svg: {known}", from.display()).into());
    };
    let mut out = vec![
        Output {
            path: L.to_string(),
            bytes: l,
            what: format!("{mark}-L.svg, the master of L"),
        },
        Output {
            path: M.to_string(),
            bytes: m,
            what: format!("{mark}-M.svg, the master of M"),
        },
        Output {
            path: S.to_string(),
            bytes: s,
            what: format!("{mark}-S.svg, the master of S"),
        },
    ];
    if let Some(banner) = read("banner") {
        out.push(Output {
            path: BANNER.to_string(),
            bytes: banner,
            what: format!("{mark}-banner.svg, the README's banner"),
        });
    }
    Ok(out)
}

/// `mark = "placeholder"` in lyrn.toml becomes `chosen`; whether it did.
fn mark_the_choice(root: &Path) -> Result<bool, Box<dyn Error>> {
    let file = root.join("lyrn.toml");
    let Ok(text) = std::fs::read_to_string(&file) else { return Ok(false) };
    let placeholder = regex::Regex::new(r#"(?m)^mark\s*=\s*"placeholder""#).expect("a valid pattern");
    if !placeholder.is_match(&text) {
        return Ok(false);
    }
    std::fs::write(&file, placeholder.replace(&text, "mark = \"chosen\"").as_bytes())?;
    Ok(true)
}

/// The mark as the repository's masters draw it.
pub fn read_mark(root: &Path) -> Result<Mark, Box<dyn Error>> {
    let read = |path: &str| std::fs::read(root.join(path)).ok();
    let (Some(l), Some(m), Some(s)) = (read(L), read(M), read(S)) else {
        return Err("no masters in assets/: the mark's levels live there as logo.svg (L), logo-m.svg (M) and logo-s.svg (S) - `lyrn assets --from <directory>` takes them in from the mark's generator".into());
    };
    let banner = read(BANNER);
    let masters = Masters {
        l: &l,
        m: &m,
        s: &s,
        banner: banner.as_deref(),
    };
    Ok(Mark::read(&masters, [L, M, S, BANNER])?)
}

/// Everything drawn from the mark, and where: the canonical set in `assets/`,
/// the icons the form's build reads, and every copy the repository already
/// keeps - an `.ico`, a favicon, a site's header, a touch icon, the icons a
/// web app's manifest lists. A copy is kept where it is rather than created
/// where nothing reads it.
pub fn outputs(repo: &Repo, mark: &Mark, form: Option<Form>, name: &str) -> Vec<Output> {
    let mut out: BTreeMap<String, Output> = BTreeMap::new();
    let mut add = |path: String, bytes: Vec<u8>, what: String| {
        out.entry(path.clone()).or_insert(Output { path, bytes, what });
    };
    let ico_what = "every size from its level: L 256-64, M 48-32, S 24-16, the largest first".to_string();

    // The canon: what every repository of the line keeps beside its masters.
    // The favicon is the one exception to the level rule: a browser draws it
    // into 16px of tab whatever size the file is, and only S survives that.
    add("assets/favicon-32.png".into(), mark.s.png(32), "S at 32px, the favicon's fallback".into());
    add("assets/apple-touch-icon.png".into(), mark.png(180), "L at 180px".into());
    add("assets/logo-512.png".into(), mark.png(512), "L at 512px, for the places outside".into());
    if let Some(preview) = mark.social_preview() {
        add(SOCIAL.into(), preview, "the banner's artwork on the plate, 1280x640".into());
    }

    // What the form's build reads its icons from.
    let has_dir = |dir: &str| repo.root.join(dir).is_dir();
    match form {
        Some(Form::Cli) => add("assets/icon.ico".into(), mark.ico(), ico_what.clone()),
        // `cargo install` builds the published crate, which carries only what
        // is under its own directory.
        Some(Form::Workspace) if has_dir(&format!("crates/{name}")) => add(format!("crates/{name}/assets/icon.ico"), mark.ico(), ico_what.clone()),
        Some(Form::Desktop) if has_dir("src-tauri") => {
            add("src-tauri/icons/icon.ico".into(), mark.ico(), ico_what.clone());
            // Tauri bundles this one for every platform but Windows.
            add("src-tauri/icons/icon.png".into(), mark.png(512), "L at 512px, the bundle's icon".into());
        }
        _ => {}
    }

    // The copies the repository keeps.
    for path in repo.files.iter().filter(|p| !is_test_material(p) && !in_a_template(p)) {
        let file = path.rsplit('/').next().unwrap_or(path);
        if path.to_ascii_lowercase().ends_with(".ico") {
            add(path.clone(), mark.ico(), ico_what.clone());
        } else if file == "favicon.svg" && !path.starts_with("assets/") {
            add(
                path.clone(),
                mark.s.source().to_vec(),
                "a copy of the S master: a favicon reads in a tab".into(),
            );
        } else if path.ends_with("src/assets/logo.svg") && !path.starts_with("assets/") {
            add(path.clone(), mark.l.source().to_vec(), "a copy of the L master: a site's header".into());
        } else if file == "apple-touch-icon.png" {
            add(path.clone(), mark.png(180), "L at 180px".into());
        } else if path.starts_with("src-tauri/icons/") && file.ends_with(".png") {
            if let Some(size) = square_png(&repo.root.join(path)) {
                add(path.clone(), mark.png(size), format!("{} at {size}px", level_name(size)));
            }
        } else if file == "manifest.webmanifest" {
            for (icon, size) in manifest_icons(repo, path) {
                add(icon, mark.png(size), format!("{} at {size}px, listed by {path}", level_name(size)));
            }
        }
    }
    out.into_values().collect()
}

fn level_name(size: u32) -> &'static str {
    match Level::for_size(size) {
        Level::L => "L",
        Level::M => "M",
        Level::S => "S",
    }
}

/// A template's files belong to the projects it writes, not to this one.
fn in_a_template(path: &str) -> bool {
    path.split('/').any(|p| p == "templates" || p == "template")
}

/// The side of a square PNG, read from its header.
fn square_png(file: &Path) -> Option<u32> {
    let bytes = std::fs::read(file).ok()?;
    let header = bytes.get(16..24).filter(|_| bytes.starts_with(b"\x89PNG"))?;
    let width = u32::from_be_bytes(header[0..4].try_into().ok()?);
    let height = u32::from_be_bytes(header[4..8].try_into().ok()?);
    (width == height && width > 0).then_some(width)
}

/// The square icons a web app manifest lists, as repository paths, with
/// their sizes. A maskable icon is drawn with room for the mask, which the
/// mark has not got: it is left as it is.
fn manifest_icons(repo: &Repo, manifest: &str) -> Vec<(String, u32)> {
    let Some(json) = repo.read_text(manifest).and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok()) else {
        return Vec::new();
    };
    let dir = manifest.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
    let icons = json.get("icons").and_then(|i| i.as_array()).cloned().unwrap_or_default();
    icons
        .iter()
        .filter(|icon| !icon.get("purpose").and_then(|p| p.as_str()).is_some_and(|p| p.contains("maskable")))
        .filter_map(|icon| {
            let src = icon.get("src")?.as_str()?;
            let (width, height) = icon.get("sizes")?.as_str()?.split_once('x')?;
            let size: u32 = width.parse().ok()?;
            if height.parse::<u32>().ok()? != size || !src.ends_with(".png") || src.contains("://") {
                return None;
            }
            let src = src.trim_start_matches('/');
            Some((if dir.is_empty() { src.to_string() } else { format!("{dir}/{src}") }, size))
        })
        .collect()
}
