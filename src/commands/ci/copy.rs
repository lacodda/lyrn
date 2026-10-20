//! The clean copy a run happens in: the commit's files and nothing else, the
//! way `actions/checkout` hands them to a runner.
//!
//! A clone sharing the repository's objects, kept in the user's cache from
//! one run to the next. Everything a commit does not track is cleaned away
//! before each run - an untracked file the build needs is exactly what CI
//! would not have - except `target/` and `node_modules/`, the build and the
//! install CI keeps in its own caches too.

use std::path::{Path, PathBuf};
use std::process::Command as Process;

/// Where this repository's copy, logs and runner directories live.
pub fn work_dir(root: &Path) -> PathBuf {
    let name = root.file_name().map_or_else(|| "repository".to_string(), |n| n.to_string_lossy().into_owned());
    // A stable hash of the path, so the same repository finds the same copy.
    let hash = crate::hash::fnv(root.display().to_string().to_lowercase().as_bytes());
    cache_root().join(format!("{name}-{:08x}", hash as u32))
}

/// The user's cache directory, as each system keeps one.
fn cache_root() -> PathBuf {
    let var = |name: &str| std::env::var_os(name).filter(|v| !v.is_empty()).map(PathBuf::from);
    let base = if cfg!(windows) {
        var("LOCALAPPDATA").unwrap_or_else(std::env::temp_dir)
    } else if cfg!(target_os = "macos") {
        var("HOME").map(|h| h.join("Library").join("Caches")).unwrap_or_else(std::env::temp_dir)
    } else {
        var("XDG_CACHE_HOME")
            .or_else(|| var("HOME").map(|h| h.join(".cache")))
            .unwrap_or_else(std::env::temp_dir)
    };
    base.join("lyrn").join("ci")
}

/// The commit `head` checked out clean in `work`, and the path of it.
pub fn prepare(root: &Path, work: &Path, head: &str, origin: Option<&str>) -> Result<PathBuf, String> {
    let checkout = work.join("checkout");
    if checkout.join(".git").exists() && git(&checkout, &["cat-file", "-e", &format!("{head}^{{commit}}")]).is_err() {
        // A copy whose shared objects went missing - the repository moved -
        // is started again rather than repaired.
        std::fs::remove_dir_all(&checkout).map_err(|e| format!("could not clear the old copy at {}: {e}", checkout.display()))?;
    }
    if !checkout.join(".git").exists() {
        std::fs::create_dir_all(work).map_err(|e| format!("could not create {}: {e}", work.display()))?;
        let source = root.display().to_string();
        let target = checkout.display().to_string();
        git(work, &["clone", "--quiet", "--shared", "--no-checkout", &source, &target])?;
    }
    git(
        &checkout,
        &["-c", "advice.detachedHead=false", "checkout", "--quiet", "--force", "--detach", head],
    )?;
    git(&checkout, &["clean", "-ffdxq", "-e", "target/", "-e", "node_modules/"])?;
    if let Some(origin) = origin {
        // A test that reads where the repository lives reads GitHub's
        // address on CI, and should read the same here.
        git(&checkout, &["remote", "set-url", "origin", origin])?;
    }
    Ok(checkout)
}

fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let out = Process::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .map_err(|e| format!("git did not start: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(format!("`git {}` failed: {}", args.join(" "), String::from_utf8_lossy(&out.stderr).trim()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_repository_finds_the_same_copy_every_time() {
        let a = work_dir(Path::new("/src/demo"));
        assert_eq!(a, work_dir(Path::new("/src/demo")));
        assert_ne!(a, work_dir(Path::new("/src/other/demo")));
        assert!(a.file_name().unwrap().to_string_lossy().starts_with("demo-"));
    }
}
