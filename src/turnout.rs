//! turnout, the line's switchyard: the apps a machine works on, the servers
//! they deploy to, and the deploy itself - built, uploaded, restarted and
//! checked, rolled back when the check fails.
//!
//! What a project says about itself to turnout lives in its `turnout.json`:
//! its commands, the directory a deploy uploads, and its stand - the port it
//! answers on, how it is put to work there, how its health is asked. Which
//! server the stand is on, who logs in and where the files land are the
//! machine's, and stay in turnout's catalog. lyrn asks turnout through its
//! command line, the way rigger is asked, and keeps no copy of what it is told.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::{Command as Process, ExitStatus, Stdio};

use serde::{Deserialize, Serialize};

/// The file a project describes itself to turnout in.
pub const FILE: &str = "turnout.json";

/// Where a new project's stand answers: the first port of this range no
/// stand of the machine is known to hold. Above turnout's own ranges (the
/// gateway's 7100-7199, the dev servers' 5100-5199), and clear of the 8080s
/// a stand set up by hand tends to take.
pub const STAND_PORTS: std::ops::RangeInclusive<u16> = 8100..=8199;

/// The variable every command on a stand is given its port in.
pub const PORT_VARIABLE: &str = "STAND_PORT";

/// What a project says about itself to turnout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Described {
    /// The app's name in turnout's catalog.
    pub name: String,
    /// Each of turnout's roles - `dev`, `build`, `test`, `lint` - and any
    /// other command, by name.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub commands: BTreeMap<String, String>,
    /// What a deploy uploads, relative to the project.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dist: Option<String>,
    /// The variable that carries the gateway's address to the app.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env_var: Option<String>,
    /// The stand the project runs on, when it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stand: Option<Stand>,
}

/// A stand: what runs it, on whichever server turnout puts it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stand {
    /// The port it answers on. Every command below is given it as
    /// `STAND_PORT`, which is what the compose file publishes.
    pub port: u16,
    /// What puts an upload to work, run in the directory it landed in.
    pub restart: String,
    /// The path that says the stand works, under its address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub health: Option<String>,
    /// What builds the stand's image there without replacing what runs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    /// What fills the stand with made-up data for a demonstration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<String>,
    /// Files of the stand's own, by path in the deploy directory, and the
    /// variables each has to hold: made there once, never uploaded.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub secrets: BTreeMap<String, Vec<String>>,
}

impl Described {
    /// The `turnout.json` at `root`, if the project has one.
    pub fn read(root: &Path) -> Result<Option<Described>, String> {
        let path = root.join(FILE);
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(format!("{FILE} could not be read: {e}")),
        };
        let described: Described = serde_json::from_str(&text).map_err(|e| format!("{FILE} does not say what turnout reads: {e}"))?;
        if described.name.trim().is_empty() {
            return Err(format!("{FILE} names no app: `name` is empty"));
        }
        Ok(Some(described))
    }

    /// A stand command, given the stand's port the way the compose file
    /// reads it.
    pub fn on_stand(stand: &Stand, command: &str) -> String {
        format!("{PORT_VARIABLE}={} {command}", stand.port)
    }
}

/// Why turnout gave no answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trouble {
    /// There is no turnout on this machine.
    Missing,
    /// turnout ran and refused, in its own words.
    Refused(String),
}

impl fmt::Display for Trouble {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Trouble::Missing => write!(f, "turnout is not installed here"),
            Trouble::Refused(said) => write!(f, "{said}"),
        }
    }
}

impl std::error::Error for Trouble {}

/// The program asked: `turnout`, unless `LYRN_TURNOUT` names another - which
/// only the tests should, to stand a catalog of their own in for yours.
fn program() -> OsString {
    std::env::var_os("LYRN_TURNOUT").filter(|v| !v.is_empty()).unwrap_or_else(|| "turnout".into())
}

/// `turnout <args>`; its standard output, or why there is none.
pub fn ask(args: &[&str]) -> Result<String, Trouble> {
    let out = Process::new(program())
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => Trouble::Missing,
            _ => Trouble::Refused(format!("turnout could not be run: {error}")),
        })?;
    if !out.status.success() {
        let said = String::from_utf8_lossy(&out.stderr);
        let said = said.trim().strip_prefix("error: ").unwrap_or(said.trim());
        return Err(Trouble::Refused(format!("`turnout {}`: {said}", shown(args))));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// `turnout <args>` in this terminal: its output is the person's to read -
/// a deploy's checklist, a command's answer on the stand.
pub fn run(args: &[&str]) -> Result<ExitStatus, Trouble> {
    Process::new(program()).args(args).status().map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => Trouble::Missing,
        _ => Trouble::Refused(format!("turnout could not be run: {error}")),
    })
}

/// The arguments as a person would type them.
fn shown(args: &[&str]) -> String {
    args.iter()
        .map(|a| if a.contains(' ') { format!("\"{a}\"") } else { (*a).to_string() })
        .collect::<Vec<_>>()
        .join(" ")
}

/// An app in turnout's catalog.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct App {
    pub name: String,
    pub path: PathBuf,
    #[serde(default)]
    pub commands: BTreeMap<String, String>,
    #[serde(default)]
    pub dist: Option<String>,
    #[serde(default)]
    pub env_var: Option<String>,
    /// The server the app is bound to: where a deploy with no target goes.
    #[serde(default)]
    pub server: Option<String>,
}

/// A deploy route: an app, the server it goes to, the path it lands in.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Target {
    pub name: String,
    pub app: String,
    pub server: String,
    pub path: String,
    pub dir: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub health: Option<Health>,
}

/// How a target's stand is asked whether it works.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Health {
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub wait: Option<u64>,
}

/// A server turnout deploys on.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Server {
    pub name: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub ssh_host: Option<String>,
    /// `posix` or `windows`: what the server's commands run in.
    #[serde(default)]
    pub shell: Option<String>,
}

impl Server {
    /// The host the server's stands are reached at: its URL's, or failing
    /// that the one SSH goes to.
    pub fn host(&self) -> Option<String> {
        self.url
            .as_deref()
            .and_then(|url| authority(url).map(|(host, _)| host))
            .or_else(|| self.ssh_host.clone())
    }
}

/// A named directory on a server, with what runs after a deploy writes it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Place {
    pub name: String,
    pub dir: String,
    #[serde(default)]
    pub restart: Option<String>,
}

fn list<T: for<'de> Deserialize<'de>>(what: &str) -> Result<Vec<T>, Trouble> {
    let json = ask(&[what, "list", "--json"])?;
    serde_json::from_str(&json).map_err(|e| Trouble::Refused(format!("turnout's {what} list is not what lyrn reads: {e}")))
}

pub fn apps() -> Result<Vec<App>, Trouble> {
    list("app")
}

pub fn targets() -> Result<Vec<Target>, Trouble> {
    list("target")
}

pub fn servers() -> Result<Vec<Server>, Trouble> {
    list("server")
}

pub fn places() -> Result<Vec<Place>, Trouble> {
    list("path")
}

/// The host and port of a URL: `http://pi:8082/x` is `pi` and `8082`.
pub fn authority(url: &str) -> Option<(String, Option<u16>)> {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    let authority = rest.split(['/', '?', '#']).next()?;
    let authority = authority.rsplit_once('@').map_or(authority, |(_, a)| a);
    if authority.is_empty() {
        return None;
    }
    if let Some(bracketed) = authority.strip_prefix('[') {
        let (host, after) = bracketed.split_once(']')?;
        return Some((host.to_string(), after.strip_prefix(':').and_then(|p| p.parse().ok())));
    }
    match authority.rsplit_once(':') {
        Some((host, port)) => Some((host.to_string(), port.parse().ok())),
        None => Some((authority.to_string(), None)),
    }
}

/// The ports the stands this machine knows answer on: every address turnout
/// holds, and every port the line's projects have already been given.
pub fn taken_stand_ports() -> BTreeSet<u16> {
    let mut taken = BTreeSet::new();
    if let Ok(targets) = targets() {
        let addresses = targets.into_iter().flat_map(|t| [t.address, t.url]).flatten();
        taken.extend(addresses.filter_map(|a| authority(&a)?.1));
    }
    if let Ok(servers) = servers() {
        taken.extend(servers.into_iter().filter_map(|s| authority(s.url.as_deref()?)?.1));
    }
    // A project made but not yet stood up has its port only in its file.
    if let Ok((repositories, _)) = crate::rigger::line() {
        for (_, path) in repositories {
            if let Ok(Some(described)) = Described::read(&path) {
                taken.extend(described.stand.map(|s| s.port));
            }
        }
    }
    taken
}

/// The first stand port none of `taken` holds.
pub fn free_stand_port(taken: &BTreeSet<u16>) -> u16 {
    STAND_PORTS.clone().find(|p| !taken.contains(p)).unwrap_or(*STAND_PORTS.start())
}

/// What registering a project with turnout came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Registered {
    /// Added to the catalog just now.
    Added,
    /// The catalog had it at this path, and now says what the file says.
    Kept { changed: Vec<String> },
}

/// Put the project at `root` into turnout's catalog as `described` says, or
/// bring the entry already there in line with it.
///
/// No gateway port: a project that describes itself here talks to its own
/// stand, and a port would have turnout write a dotenv file into a repository
/// that has no use for one.
pub fn register(root: &Path, described: &Described) -> Result<Registered, Trouble> {
    let root = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let root = strip_verbatim(&root);
    let known = apps()?;
    if let Some(app) = known.iter().find(|a| a.name == described.name) {
        if !same_place(&app.path, &root) {
            return Err(Trouble::Refused(format!(
                "turnout already has an app called `{}`, at {} - `turnout app remove {}` frees the name",
                app.name,
                app.path.display(),
                app.name
            )));
        }
        return reconcile(app, described).map(|changed| Registered::Kept { changed });
    }
    let args = add_args(&root.display().to_string(), described);
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    ask(&args).map(|_| Registered::Added)
}

/// `turnout app add` for the project at `path`, as its file describes it.
fn add_args(path: &str, described: &Described) -> Vec<String> {
    let mut args: Vec<String> = ["app", "add", described.name.as_str(), "--path", path, "--port", "0"]
        .map(String::from)
        .to_vec();
    if let Some(dist) = &described.dist {
        args.extend(["--dist".to_string(), dist.clone()]);
    }
    if let Some(var) = &described.env_var {
        args.extend(["--env-var".to_string(), var.clone()]);
    }
    for (name, line) in &described.commands {
        args.extend(["--command".to_string(), format!("{name}={line}")]);
    }
    args
}

/// Bring an app's entry in line with its file; what changed, by field.
fn reconcile(app: &App, described: &Described) -> Result<Vec<String>, Trouble> {
    let (args, changed) = edit_args(app, described);
    if changed.is_empty() {
        return Ok(changed);
    }
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    ask(&args).map(|_| changed)
}

/// `turnout app edit` for what the entry says otherwise than the file, and
/// what that is, in words. A command the entry has and the file does not is
/// somebody's own, and stays.
fn edit_args(app: &App, described: &Described) -> (Vec<String>, Vec<String>) {
    let mut args = vec!["app".to_string(), "edit".to_string(), app.name.clone()];
    let mut changed = Vec::new();
    for (name, line) in &described.commands {
        if app.commands.get(name) != Some(line) {
            args.extend(["--command".to_string(), format!("{name}={line}")]);
            changed.push(format!("the {name} command"));
        }
    }
    if let Some(dist) = described.dist.as_ref().filter(|d| app.dist.as_ref() != Some(*d)) {
        args.extend(["--dist".to_string(), dist.clone()]);
        changed.push("what a deploy uploads".to_string());
    }
    if let Some(var) = described.env_var.as_ref().filter(|v| app.env_var.as_ref() != Some(*v)) {
        args.extend(["--env-var".to_string(), var.clone()]);
        changed.push("the gateway's variable".to_string());
    }
    (args, changed)
}

/// `\\?\C:\x` is `C:\x` to everyone but the Windows API.
fn strip_verbatim(path: &Path) -> PathBuf {
    let text = path.display().to_string();
    match text.strip_prefix(r"\\?\") {
        Some(rest) if !rest.starts_with("UNC") => PathBuf::from(rest),
        _ => path.to_path_buf(),
    }
}

/// Whether two paths are one directory: by their components once resolved,
/// and on Windows regardless of case.
fn same_place(a: &Path, b: &Path) -> bool {
    let resolve = |p: &Path| strip_verbatim(&std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf()));
    let parts = |p: &Path| -> Vec<String> {
        p.components()
            .map(|c| {
                let part = c.as_os_str().to_string_lossy().into_owned();
                if cfg!(windows) { part.to_lowercase() } else { part }
            })
            .collect()
    };
    parts(&resolve(a)) == parts(&resolve(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_url_s_host_and_port_are_its_authority() {
        assert_eq!(authority("http://pi:8082"), Some(("pi".into(), Some(8082))));
        assert_eq!(authority("http://pi:8082/api/health?x=1"), Some(("pi".into(), Some(8082))));
        assert_eq!(authority("https://efema.pi"), Some(("efema.pi".into(), None)));
        assert_eq!(authority("https://me@host.example:22/"), Some(("host.example".into(), Some(22))));
        assert_eq!(authority("http://[::1]:8100/"), Some(("::1".into(), Some(8100))));
        assert_eq!(authority("http://"), None);
    }

    #[test]
    fn the_first_port_nothing_holds_is_the_stand_s() {
        assert_eq!(free_stand_port(&BTreeSet::new()), 8100);
        assert_eq!(free_stand_port(&[8080, 8100, 8101, 8103].into()), 8102);
        let every: BTreeSet<u16> = STAND_PORTS.collect();
        assert_eq!(free_stand_port(&every), 8100, "a full range still names a port rather than none");
    }

    #[test]
    fn a_description_reads_what_the_service_form_writes() {
        let text = r#"{
            "name": "demo-svc",
            "commands": {"build": "sh tools/stand/stage.sh target/stand", "dev": "cargo run"},
            "dist": "target/stand",
            "stand": {
                "port": 8100,
                "restart": "docker compose -f deploy/compose.yml up -d --build",
                "health": "/api/health",
                "secrets": {"deploy/.env": ["POSTGRES_PASSWORD"]}
            }
        }"#;
        let described: Described = serde_json::from_str(text).unwrap();
        let stand = described.stand.as_ref().unwrap();
        assert_eq!(stand.port, 8100);
        assert_eq!(stand.secrets["deploy/.env"], ["POSTGRES_PASSWORD"]);
        assert_eq!(
            Described::on_stand(stand, &stand.restart),
            "STAND_PORT=8100 docker compose -f deploy/compose.yml up -d --build"
        );
    }

    /// A misspelt key is a stand that does something other than it says.
    #[test]
    fn a_key_turnout_json_does_not_have_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(FILE),
            r#"{"name": "x", "stand": {"port": 8100, "restart": "up", "helth": "/"}}"#,
        )
        .unwrap();
        let error = Described::read(dir.path()).unwrap_err();
        assert!(error.contains("helth"), "{error}");
        assert_eq!(Described::read(&dir.path().join("missing")).unwrap(), None);
    }

    fn described() -> Described {
        Described {
            name: "demo-svc".into(),
            commands: [("build", "sh tools/stand/stage.sh target/stand"), ("dev", "cargo run")]
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .into(),
            dist: Some("target/stand".into()),
            env_var: None,
            stand: None,
        }
    }

    /// No gateway port: turnout would write its address into a dotenv file
    /// of a repository that talks to its own stand.
    #[test]
    fn a_project_is_added_as_its_file_describes_it_without_a_gateway_port() {
        let args = add_args("/src/demo-svc", &described());
        assert_eq!(
            args,
            [
                "app",
                "add",
                "demo-svc",
                "--path",
                "/src/demo-svc",
                "--port",
                "0",
                "--dist",
                "target/stand",
                "--command",
                "build=sh tools/stand/stage.sh target/stand",
                "--command",
                "dev=cargo run"
            ]
        );
    }

    #[test]
    fn an_entry_is_edited_only_where_it_says_otherwise_than_the_file() {
        let app = App {
            name: "demo-svc".into(),
            path: "/src/demo-svc".into(),
            commands: [("build", "pnpm build"), ("dev", "cargo run"), ("logs", "docker compose logs")]
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .into(),
            dist: Some("target/stand".into()),
            env_var: Some("TURNOUT_GATEWAY_URL".into()),
            server: None,
        };
        let (args, changed) = edit_args(&app, &described());
        assert_eq!(args, ["app", "edit", "demo-svc", "--command", "build=sh tools/stand/stage.sh target/stand"]);
        assert_eq!(changed, ["the build command"]);

        let same = App {
            commands: described().commands,
            ..app
        };
        assert!(edit_args(&same, &described()).1.is_empty());
    }

    #[test]
    fn a_server_s_stands_are_reached_at_its_url_s_host() {
        let server = |url: Option<&str>, ssh: Option<&str>| Server {
            name: "pi".into(),
            url: url.map(str::to_string),
            ssh_host: ssh.map(str::to_string),
            shell: Some("posix".into()),
        };
        assert_eq!(server(Some("http://pi:8082"), Some("10.0.0.2")).host().as_deref(), Some("pi"));
        assert_eq!(server(None, Some("10.0.0.2")).host().as_deref(), Some("10.0.0.2"));
        assert_eq!(server(None, None).host(), None);
    }
}
