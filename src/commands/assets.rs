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
    for output in outputs(&repo, &mark) {
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
/// the icons the repository's builds read, and every copy it already
/// keeps - an `.ico`, a favicon, a site's header, a touch icon, the icons a
/// web app's manifest lists. A copy is kept where it is rather than created
/// where nothing reads it.
pub fn outputs(repo: &Repo, mark: &Mark) -> Vec<Output> {
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

    // What a build reads: every `.ico` a build script embeds, and every icon
    // a Tauri app's bundle lists. Written whether or not it is there yet - a
    // build that reads a missing icon fails - and nowhere else: an icon no
    // build reads is a copy that drifts from the masters unnoticed.
    for path in build_icons(repo) {
        add(path, mark.ico(), ico_what.clone());
    }
    for (path, size) in tauri_icons(repo) {
        match size {
            None => add(path, mark.ico(), ico_what.clone()),
            Some(size) => add(path, mark.png(size), format!("{} at {size}px, the bundle's icon", level_name(size))),
        }
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
        } else if (path.starts_with("src-tauri/icons/") || path.contains("/src-tauri/icons/")) && file.ends_with(".png") {
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

/// The `.ico` files the repository's build scripts embed: every string in a
/// `build.rs` that names one, taken from the script's crate, as cargo runs
/// it - `"assets/icon.ico"` beside the crate, or `"../../assets/icon.ico"`
/// at the top of a workspace.
fn build_icons(repo: &Repo) -> Vec<String> {
    let mut out = Vec::new();
    for script in repo.files_named("build.rs").filter(|p| !is_test_material(p) && !in_a_template(p)) {
        let Some(text) = repo.read_text(script) else { continue };
        let dir = script.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
        for named in icons_named_in(&text) {
            if let Some(path) = resolve(dir, &named)
                && !out.contains(&path)
            {
                out.push(path);
            }
        }
    }
    out
}

/// The `.ico` paths a script's strings name. A path has no spaces: a string
/// with them is a message about the icon -
/// `expect("the mark is missing from assets/icon.ico")` - not its place.
fn icons_named_in(text: &str) -> Vec<String> {
    let named = regex::Regex::new(r#""([^"\s]*\.ico)""#).expect("a valid pattern");
    named.captures_iter(text).map(|c| c[1].to_string()).collect()
}

/// The icons a Tauri app's bundle lists, with the size each PNG is drawn at:
/// from its name - `32x32.png`, `128x128@2x.png` - else from the file, else
/// 512. `None` is an `.ico`; Apple's `.icns` is not drawn here.
fn tauri_icons(repo: &Repo) -> Vec<(String, Option<u32>)> {
    let mut out = Vec::new();
    for config in repo.files_named("tauri.conf.json").filter(|p| !is_test_material(p) && !in_a_template(p)) {
        let Some(json) = repo.read_text(config).and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok()) else {
            continue;
        };
        let dir = config.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
        let icons = json.pointer("/bundle/icon").and_then(|i| i.as_array()).cloned().unwrap_or_default();
        for icon in icons.iter().filter_map(|i| i.as_str()) {
            let Some(path) = resolve(dir, icon) else { continue };
            let lower = path.to_ascii_lowercase();
            if lower.ends_with(".ico") {
                out.push((path, None));
            } else if lower.ends_with(".png") {
                let size = size_in_name(&path).or_else(|| square_png(&repo.root.join(&path))).unwrap_or(512);
                out.push((path, Some(size)));
            }
        }
    }
    out
}

/// `32x32.png` is 32, `128x128@2x.png` is 256.
fn size_in_name(path: &str) -> Option<u32> {
    let name = path.rsplit('/').next()?.strip_suffix(".png")?;
    let (side, scale) = match name.split_once('@') {
        Some((side, scale)) => (side, scale.strip_suffix('x')?.parse::<u32>().ok()?),
        None => (name, 1),
    };
    let (width, height) = side.split_once('x')?;
    let width: u32 = width.parse().ok()?;
    (height.parse::<u32>().ok()? == width).then_some(width * scale)
}

/// A path written relative to `dir`, as a path of the repository; `None` when
/// it leaves the repository or is absolute.
fn resolve(dir: &str, relative: &str) -> Option<String> {
    if relative.starts_with('/') || relative.contains(':') || relative.contains("${") || relative.contains('{') {
        return None;
    }
    let mut parts: Vec<&str> = dir.split('/').filter(|p| !p.is_empty()).collect();
    for part in relative.split(['/', '\\']) {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            other => parts.push(other),
        }
    }
    Some(parts.join("/"))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_is_read_from_where_the_build_runs() {
        assert_eq!(resolve("", "assets/icon.ico").as_deref(), Some("assets/icon.ico"));
        assert_eq!(resolve("crates/cli", "assets/icon.ico").as_deref(), Some("crates/cli/assets/icon.ico"));
        assert_eq!(resolve("crates/nooma", "../../assets/icon.ico").as_deref(), Some("assets/icon.ico"));
        assert_eq!(resolve("src-tauri", "./icons/icon.ico").as_deref(), Some("src-tauri/icons/icon.ico"));
        assert_eq!(resolve("", "../elsewhere/icon.ico"), None, "outside the repository");
        assert_eq!(resolve("", "C:/icons/icon.ico"), None);
        assert_eq!(resolve("", "{dir}/icon.ico"), None, "a path built at run time");
    }

    #[test]
    fn a_build_script_names_its_icon_and_a_message_does_not() {
        let script = r#"
            const ICON: &str = "assets/icon.ico";
            let icon = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/icon.ico");
            std::fs::metadata(&icon).expect("the line mark is missing from assets/icon.ico");
        "#;
        assert_eq!(icons_named_in(script), ["assets/icon.ico", "../../assets/icon.ico"]);
    }

    #[test]
    fn a_tauri_icon_is_drawn_at_the_size_its_name_says() {
        assert_eq!(size_in_name("src-tauri/icons/32x32.png"), Some(32));
        assert_eq!(size_in_name("src-tauri/icons/128x128@2x.png"), Some(256));
        assert_eq!(size_in_name("src-tauri/icons/icon.png"), None);
        assert_eq!(size_in_name("src-tauri/icons/Square30x30Logo.png"), None);
    }
}
