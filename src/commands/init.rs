//! `lyrn init`: a project started in a directory that already exists.
//!
//! The common case is a repository created on GitHub and cloned empty, or a
//! directory made by hand before deciding what goes in it. `lyrn new` refuses
//! both - its destination must be empty or absent - so `init` is the same
//! generation with the other placement: the directory has to exist, whatever
//! is already in it stays, and a single planned file that is already there
//! stops the whole run before anything is written.

use std::error::Error;
use std::path::{Path, PathBuf};

use crate::cli::InitArgs;
use crate::commands::new::{Wanted, generate_project};
use crate::model::Placement;
use crate::naming;

pub fn run(args: InitArgs) -> Result<(), Box<dyn Error>> {
    let root = args.path.clone().unwrap_or_else(|| PathBuf::from("."));
    if !root.is_dir() {
        return Err(format!(
            "`{}` is not a directory - `lyrn init` starts a project in one that exists; `lyrn new` creates it",
            root.display()
        )
        .into());
    }

    let name = match &args.name {
        Some(name) => name.clone(),
        None => name_of(&root)?,
    };
    generate_project(&Wanted::from_project(&name, &args.project), &args.project, &root, Placement::IntoExisting)
}

/// The project name a directory implies: its own name, if that is a valid
/// one. `.` has no name of its own, so the real path is asked for.
pub fn name_of(dir: &Path) -> Result<String, Box<dyn Error>> {
    let full = dir.canonicalize()?;
    let Some(name) = full.file_name().map(|n| n.to_string_lossy().into_owned()) else {
        return Err(format!("`{}` has no name to take - pass `--name`", dir.display()).into());
    };
    naming::validate_name(&name).map_err(|reason| format!("the directory is called `{name}`, which is not a project name: {reason} - pass `--name`"))?;
    Ok(name)
}
