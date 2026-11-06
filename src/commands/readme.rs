//! `lyrn readme snap`: the README's transcripts, taken again from the build.
//!
//! A transcript is the README's strongest claim: this is what happens when you
//! type this. Written by hand, or pasted once, it goes stale with the first
//! change to the output, and nothing fails. The release rituals of the line
//! ended with someone running the commands and pasting by hand, which is a step
//! that gets skipped on the day it matters.
//!
//! A block is taken again only when it asks to be, by a comment on the line
//! above it: most console blocks in a README are usage, with placeholders, or
//! commands nobody should run for a screenshot. Marked blocks run in order in a
//! scratch directory, one shell after another, keeping the directory the last
//! one ended in - they are one person's afternoon, not separate examples.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command as Process;

use regex::Regex;

use crate::cli::ReadmeCommand;
use crate::commands::check::commands::binary;
use crate::commands::check::product::Product;
use crate::commands::doctor::Repo;

const FILE: &str = "README.md";

pub fn run(command: ReadmeCommand) -> Result<bool, Box<dyn Error>> {
    match command {
        ReadmeCommand::Snap { path, check } => snap(path, check),
    }
}

fn snap(path: Option<PathBuf>, check: bool) -> Result<bool, Box<dyn Error>> {
    let root = path.unwrap_or_else(|| PathBuf::from("."));
    if !root.is_dir() {
        return Err(format!("`{}` is not a directory - `lyrn readme snap` reads a repository's README", root.display()).into());
    }
    let repo = Repo::read(&root);
    let file = repo.root.join(FILE);
    let text = std::fs::read_to_string(&file).map_err(|e| format!("{FILE}: {e}"))?;
    let blocks = blocks_in(&text)?;
    if blocks.is_empty() {
        println!("{FILE} marks no transcript: `<!-- snap -->` on the line above a console block asks for one.");
        return Ok(true);
    }

    let scratch = tempfile::tempdir()?;
    let dir = scratch.path().canonicalize()?;
    let mut session = Session::new(&repo, &dir)?;
    let mut taken = text.clone();
    let mut stale = Vec::new();
    // From the last to the first, so an offset still holds once a later block
    // has changed length.
    let mut rendered = Vec::new();
    for block in &blocks {
        rendered.push(session.take(block)?);
    }
    for (block, fresh) in blocks.iter().zip(&rendered).rev() {
        let old = &text[block.body.clone()];
        if old.replace("\r\n", "\n") != *fresh {
            stale.push(block.commands.first().cloned().unwrap_or_default());
            taken.replace_range(block.body.clone(), fresh);
        }
    }
    stale.reverse();

    if stale.is_empty() {
        println!("{FILE}: {} as the build prints them.", plural(blocks.len()));
        return Ok(true);
    }
    if check {
        println!("{FILE}: {} of {} no longer what the build prints:", stale.len(), plural(blocks.len()));
        println!("{}", diffy::create_patch(&text.replace("\r\n", "\n"), &taken.replace("\r\n", "\n")));
        println!("`lyrn readme snap` takes them again.");
        return Ok(false);
    }
    std::fs::write(&file, taken).map_err(|e| format!("{FILE}: {e}"))?;
    println!("{FILE}: took {} again, {} changed:", plural(blocks.len()), stale.len());
    for command in stale {
        println!("  $ {command}");
    }
    Ok(true)
}

fn plural(count: usize) -> String {
    if count == 1 {
        "1 transcript".to_string()
    } else {
        format!("{count} transcripts")
    }
}

/// One console block a comment asked to have taken again.
#[derive(Debug, PartialEq, Eq)]
struct Block {
    /// The byte range of what is between the fences.
    body: std::ops::Range<usize>,
    commands: Vec<String>,
    /// `keep`: only the lines of the output this matches go into the README.
    keep: Option<String>,
}

/// Every marked block. A mark with no console block under it is an error: it
/// would otherwise leave a transcript nobody retakes, under a comment saying
/// it is.
fn blocks_in(text: &str) -> Result<Vec<Block>, String> {
    let marker = Regex::new(r"^<!--\s*snap(?::\s*keep\s+(.+?))?\s*-->\s*$").expect("a valid pattern");
    let lines: Vec<(usize, &str)> = {
        let mut offset = 0;
        text.split_inclusive('\n')
            .map(|line| {
                let at = offset;
                offset += line.len();
                (at, line)
            })
            .collect()
    };
    let mut blocks = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let (_, line) = lines[index];
        index += 1;
        let Some(found) = marker.captures(line.trim_end()) else { continue };
        let at = index;
        let Some((_, fence)) = lines.get(index).filter(|(_, l)| l.trim_end() == "```console") else {
            return Err(format!("{FILE}:{at}: `<!-- snap -->` has no ```console block on the line below it"));
        };
        let start = lines[index].0 + fence.len();
        index += 1;
        let close = lines[index..]
            .iter()
            .position(|(_, l)| l.trim_end() == "```")
            .map(|n| n + index)
            .ok_or_else(|| format!("{FILE}:{at}: the console block never closes"))?;
        let body = start..lines[close].0;
        let commands: Vec<String> = lines[index..close]
            .iter()
            .filter_map(|(_, l)| l.trim_end().strip_prefix("$ ").map(str::to_string))
            .collect();
        if commands.is_empty() {
            return Err(format!("{FILE}:{at}: the marked block has no `$ ` line to run"));
        }
        let keep = found.get(1).map(|m| m.as_str().to_string());
        if let Some(pattern) = &keep {
            Regex::new(pattern).map_err(|e| format!("{FILE}:{at}: `keep` is not a pattern: {e}"))?;
        }
        blocks.push(Block { body, commands, keep });
        index = close + 1;
    }
    Ok(blocks)
}

/// The shell the transcripts run in: where the last command left it, with the
/// product's own build first on the PATH.
struct Session<'a> {
    bash: PathBuf,
    scratch: &'a Path,
    /// Every way a program here may print the scratch directory.
    spellings: Vec<String>,
    here: PathBuf,
    path: std::ffi::OsString,
}

impl<'a> Session<'a> {
    fn new(repo: &Repo, scratch: &'a Path) -> Result<Session<'a>, Box<dyn Error>> {
        let bash = crate::commands::ci::shell::bash().ok_or("a transcript runs in bash, and no bash is installed here (Git for Windows brings one)")?;
        let product = Product::read(repo);
        let mut dirs: Vec<PathBuf> = Vec::new();
        for (manifest, bin) in product.programs() {
            let built = binary(repo, manifest, bin).map_err(|reason| format!("`{bin}` could not be built: {reason}"))?;
            if let Some(dir) = built.parent().map(Path::to_path_buf)
                && !dirs.contains(&dir)
            {
                dirs.push(dir);
            }
        }
        dirs.extend(std::env::var_os("PATH").iter().flat_map(std::env::split_paths));
        // bash's own name for it, which on Windows is more than one rule can
        // say: Git for Windows mounts the temporary directory at `/tmp`.
        let named = Process::new(&bash)
            .args(["--noprofile", "--norc", "-c", "pwd"])
            .current_dir(scratch)
            .output()
            .map_err(|e| format!("bash did not start: {e}"))?;
        let named = String::from_utf8_lossy(&named.stdout).trim().to_string();
        Ok(Session {
            bash,
            scratch,
            spellings: spellings(scratch, &named),
            here: scratch.to_path_buf(),
            path: std::env::join_paths(dirs)?,
        })
    }

    /// The block as the build prints it now, between its fences.
    fn take(&mut self, block: &Block) -> Result<String, Box<dyn Error>> {
        let keep = block.keep.as_deref().map(Regex::new).transpose()?;
        let mut out = String::new();
        for command in &block.commands {
            println!("$ {command}");
            let printed = self.run(command)?;
            out.push_str(&format!("$ {command}\n"));
            for line in clean(&printed, &self.spellings).lines() {
                if keep.as_ref().is_none_or(|k| k.is_match(line)) {
                    out.push_str(line);
                    out.push('\n');
                }
            }
        }
        Ok(out)
    }

    /// One command, its streams as a terminal would have shown them. One that
    /// fails stops the snap with what it said: a README that shows a failure
    /// as what happens is worse than an old one.
    fn run(&mut self, command: &str) -> Result<String, Box<dyn Error>> {
        let left_in = self.scratch.join(".lyrn-snap-here");
        let at = left_in.display().to_string().replace('\\', "/");
        let script = format!("{{ {command}\n}} 2>&1\ncode=$?\n{{ pwd -W 2>/dev/null || pwd; }} > '{at}'\nexit $code\n");
        let out = Process::new(&self.bash)
            .args(["--noprofile", "--norc", "-c", &script])
            .current_dir(&self.here)
            .env("PATH", &self.path)
            .env("NO_COLOR", "1")
            .stdin(std::process::Stdio::null())
            .output()
            .map_err(|e| format!("bash did not start: {e}"))?;
        let printed = String::from_utf8_lossy(&out.stdout).into_owned();
        if !out.status.success() {
            let shown = clean(&printed, &self.spellings);
            return Err(format!("`{command}` failed, and {FILE} was left as it was:\n{shown}").into());
        }
        if let Ok(next) = std::fs::read_to_string(&left_in) {
            let next = PathBuf::from(next.trim());
            if next.is_dir() {
                self.here = next;
            }
        }
        Ok(printed)
    }
}

/// What was printed, as it reads anywhere: no colours, no carriage returns, and
/// the scratch directory written as `~`, in every spelling a program on this
/// machine may print it in.
fn clean(printed: &str, spellings: &[String]) -> String {
    let colours = Regex::new(r"\x1b\[[0-9;?]*[ -/]*[@-~]").expect("a valid pattern");
    let text = colours.replace_all(printed, "").replace("\r\n", "\n");
    let mut text: String = text
        .lines()
        .map(|line| line.rsplit('\r').next().unwrap_or(line).trim_end())
        .collect::<Vec<_>>()
        .join("\n");
    for spelling in spellings {
        text = text.replace(spelling.as_str(), "~");
    }
    text.trim_end().to_string()
}

fn spellings(scratch: &Path, in_bash: &str) -> Vec<String> {
    let native = scratch.display().to_string();
    let bare = native.strip_prefix(r"\\?\").unwrap_or(&native).to_string();
    let mut all = vec![native.clone(), bare.clone(), bare.replace('\\', "/")];
    if !in_bash.is_empty() {
        all.push(in_bash.to_string());
    }
    // The drive's mount: a program may print it where bash itself does not.
    if let Some((drive, rest)) = bare.split_once(":\\") {
        all.push(format!("/{}/{}", drive.to_lowercase(), rest.replace('\\', "/")));
    }
    all.sort_by_key(|s| std::cmp::Reverse(s.len()));
    all.dedup();
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_marked_block_is_taken() {
        let text = "```console\n$ usage <name>\n```\n\n<!-- snap -->\n```console\n$ lyrn doctor\nold\n```\n\n<!-- snap: keep ^(ok|fine) -->\n```console\n$ one\n$ two\n```\n";
        let blocks = blocks_in(text).unwrap();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].commands, ["lyrn doctor"]);
        assert_eq!(&text[blocks[0].body.clone()], "$ lyrn doctor\nold\n");
        assert_eq!(blocks[0].keep, None);
        assert_eq!(blocks[1].commands, ["one", "two"]);
        assert_eq!(blocks[1].keep.as_deref(), Some("^(ok|fine)"));
    }

    #[test]
    fn a_mark_without_its_block_is_an_error() {
        assert!(
            blocks_in("<!-- snap -->\n\n```console\n$ a\n```\n")
                .unwrap_err()
                .contains("no ```console block")
        );
        assert!(blocks_in("<!-- snap -->\n```console\n$ a\n").unwrap_err().contains("never closes"));
        assert!(
            blocks_in("<!-- snap -->\n```console\nnot a command\n```\n")
                .unwrap_err()
                .contains("no `$ ` line")
        );
        assert!(
            blocks_in("<!-- snap: keep ( -->\n```console\n$ a\n```\n")
                .unwrap_err()
                .contains("not a pattern")
        );
    }

    #[test]
    fn what_was_printed_reads_anywhere() {
        let scratch = if cfg!(windows) { Path::new(r"C:\Temp\abc") } else { Path::new("/tmp/abc") };
        let native = scratch.display().to_string();
        let printed = format!("\x1b[32mok\x1b[0m\r\nfrom {native}/app\r\nnow 50%\rnow 100%\n\n");
        let spelt = spellings(scratch, "/tmp/.tmpX");
        assert_eq!(clean(&printed, &spelt), "ok\nfrom ~/app\nnow 100%");
        assert_eq!(clean("at /tmp/.tmpX/app", &spelt), "at ~/app");
        if cfg!(windows) {
            assert_eq!(clean("at C:/Temp/abc/app and /c/Temp/abc/app", &spelt), "at ~/app and ~/app");
        }
    }
}
