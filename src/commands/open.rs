//! `lyrn open`: one of the repository's shopfronts, in the browser.
//!
//! The addresses are the repository's own - its remote, its site's config,
//! its tags, its manifests - so the page opened is the one the project points
//! at, not the one somebody remembers.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command as Process;

use crate::cli::OpenArgs;
use crate::commands::check::product::{Kind, Product};
use crate::commands::check::site::Site;
use crate::commands::doctor::Repo;
use crate::commands::repo::{release_workflow, site_address};

/// A shopfront, by the name `lyrn open` takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Place {
    /// The repository on GitHub
    Repo,
    /// The documentation site
    Docs,
    /// The CI the release waits for, on GitHub
    Ci,
    /// The newest release's page
    Release,
    /// The crate on crates.io
    Crate,
    /// The package on npm
    Npm,
}

pub fn run(args: OpenArgs) -> Result<(), Box<dyn Error>> {
    let path = args.path.clone().unwrap_or_else(|| PathBuf::from("."));
    if !path.is_dir() {
        return Err(format!("`{}` is not a directory - `lyrn open` opens a repository's pages", path.display()).into());
    }
    let repo = Repo::read(&path);
    let product = Product::read(&repo);
    let url = address(&repo, &product, args.place.unwrap_or(Place::Repo))?;
    if args.print {
        println!("{url}");
        return Ok(());
    }
    println!("opening {url}");
    browse(&url)
}

/// Where a shopfront of this repository is.
pub fn address(repo: &Repo, product: &Product, place: Place) -> Result<String, Box<dyn Error>> {
    let github = || {
        product
            .github
            .clone()
            .ok_or_else(|| "no GitHub repository is named here: `origin` points at none, and no manifest says `repository`".to_string())
    };
    let published = |kind: Kind| {
        let mut manifests: Vec<_> = product.manifests.iter().filter(|m| m.kind == kind && m.publishable).collect();
        manifests.sort_by_key(|m| (!product.owns(m.name.as_deref().unwrap_or_default()), m.path.matches('/').count()));
        manifests.first().and_then(|m| m.name.clone())
    };
    Ok(match place {
        Place::Repo => format!("https://github.com/{}", github()?),
        Place::Docs => Site::read(repo)
            .as_ref()
            .and_then(site_address)
            .ok_or("no documentation site here whose config says where it is served - `lyrn new <name> --form docs` adds one")?,
        Place::Ci => {
            let workflow = release_workflow(&repo.root).unwrap_or_else(|| "ci.yml".to_string());
            format!("https://github.com/{}/actions/workflows/{workflow}", github()?)
        }
        Place::Release => match newest_tag(&repo.root) {
            Some(tag) => format!("https://github.com/{}/releases/tag/{tag}", github()?),
            None => format!("https://github.com/{}/releases", github()?),
        },
        Place::Crate => format!(
            "https://crates.io/crates/{}",
            published(Kind::Cargo).ok_or("no crate here that crates.io would take: every Cargo.toml says `publish = false`, or there is none")?
        ),
        Place::Npm => format!(
            "https://www.npmjs.com/package/{}",
            published(Kind::Package).ok_or("no package here that npm would take: every package.json is private, or there is none")?
        ),
    })
}

/// The newest version tag, by version rather than by date.
fn newest_tag(root: &Path) -> Option<String> {
    let out = Process::new("git")
        .args(["tag", "--list", "v*", "--sort=-v:refname"])
        .current_dir(root)
        .output()
        .ok()?;
    String::from_utf8(out.stdout)
        .ok()?
        .lines()
        .map(str::trim)
        .find(|t| !t.is_empty())
        .map(str::to_string)
}

/// The address in the browser the system opens addresses with. On Windows
/// through the URL handler rather than `start`, which `cmd` would parse, and
/// an `&` in a query would end the command there.
fn browse(url: &str) -> Result<(), Box<dyn Error>> {
    let (program, args): (&str, Vec<&str>) = if cfg!(windows) {
        ("rundll32", vec!["url.dll,FileProtocolHandler", url])
    } else if cfg!(target_os = "macos") {
        ("open", vec![url])
    } else {
        ("xdg-open", vec![url])
    };
    Process::new(program)
        .args(&args)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("could not open a browser with `{program}`: {e} - `lyrn open --print` prints the address instead").into())
}
