//! `lyrn stand`: the service's stand, through turnout.
//!
//! What the stand is - the port it answers on, what puts an upload to work,
//! how its health is asked, what builds its image and what fills it for a
//! demonstration - is the project's own, and lives in its `turnout.json`.
//! Which server it is on, who logs in and where the files land are the
//! machine's, in turnout's catalog. `up` brings the catalog in line with the
//! file and deploys; turnout does the rest - the build, the upload, the
//! restart, the check, and the rollback of a deploy that fails it.
//!
//! The stand builds its own image from the source it is sent, so a Pi builds
//! for arm64 on the Pi and nothing is cross-compiled. `image` proves that
//! build there without replacing what runs; `image --ci` proves it in CI on
//! both architectures, for a stand out of reach.

use std::collections::BTreeSet;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command as Process;
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::cli::StandCommand;
use crate::turnout::{self, Described, Registered, Stand, Target, Trouble};

/// The workflow that builds and starts the image in CI.
const IMAGE_WORKFLOW: &str = "image.yml";

pub fn run(command: StandCommand) -> Result<(), Box<dyn Error>> {
    match command {
        StandCommand::Up { server, dir, path } => up(&root(path)?, server.as_deref(), dir.as_deref()),
        StandCommand::Seed { server, path } => seed(&root(path)?, server.as_deref()),
        StandCommand::Image { ci: true, path, .. } => image_in_ci(&root(path)?),
        StandCommand::Image { server, path, .. } => image(&root(path)?, server.as_deref()),
    }
}

fn root(path: Option<PathBuf>) -> Result<PathBuf, Box<dyn Error>> {
    let root = path.unwrap_or_else(|| PathBuf::from("."));
    if !root.is_dir() {
        return Err(format!("`{}` is not a directory - `lyrn stand` works on a repository", root.display()).into());
    }
    Ok(root)
}

/// The project's description and its stand.
fn described(root: &Path) -> Result<(Described, Stand), Box<dyn Error>> {
    let described = Described::read(root)?.ok_or_else(|| {
        format!(
            "no {} here: a stand is described in it - its port, what runs it, how its health is asked; the service form writes one",
            turnout::FILE
        )
    })?;
    let stand = described
        .stand
        .clone()
        .ok_or_else(|| format!("{} describes no stand: its `stand` says the port and what runs it", turnout::FILE))?;
    Ok((described, stand))
}

/// turnout's refusal, with what to do when it is not there at all.
fn told(trouble: Trouble) -> Box<dyn Error> {
    match trouble {
        Trouble::Missing => "turnout is not installed here, and a stand goes up through it - https://lacodda.github.io/turnout/".into(),
        Trouble::Refused(said) => said.into(),
    }
}

fn up(root: &Path, server: Option<&str>, dir: Option<&str>) -> Result<(), Box<dyn Error>> {
    let (described, stand) = described(root)?;
    let name = &described.name;
    match turnout::register(root, &described).map_err(told)? {
        Registered::Added => say("app", &format!("{name} is in turnout's catalog now")),
        Registered::Kept { changed } if changed.is_empty() => say("app", &format!("{name}, as {} says", turnout::FILE)),
        Registered::Kept { changed } => say("app", &format!("{name}: {} brought in line", changed.join(", "))),
    }
    let target = route(&described, &stand, server, dir)?;
    bind(name, &target)?;
    make_secrets(&target, &stand)?;

    say("deploy", &format!("turnout deploy {}", target.name));
    let status = turnout::run(&["deploy", &target.name]).map_err(told)?;
    if !status.success() {
        return Err(format!(
            "`turnout deploy {}` did not finish - its checklist above says why; a stand that fails its check is put back as it was",
            target.name
        )
        .into());
    }
    let address = target.url.or(target.address).unwrap_or_default();
    println!("\n{name} is up on {}: {address}", target.server);
    Ok(())
}

/// The app bound to the stand's server when it is bound to none: a deploy
/// that names no target - the one a release makes after it publishes - goes
/// to the server the app is bound to.
fn bind(name: &str, target: &Target) -> Result<(), Box<dyn Error>> {
    let bound = turnout::apps().map_err(told)?.into_iter().find(|a| a.name == name).and_then(|a| a.server);
    match bound {
        Some(server) if server == target.server => say("binding", &format!("{name} uses {server}, where a release deploys it")),
        Some(server) => say("binding", &format!("{name} uses {server}: a release deploys there, not to {}", target.server)),
        None => {
            turnout::ask(&["use", name, &target.server]).map_err(told)?;
            say("binding", &format!("{name} uses {} now, where a release deploys it", target.server));
        }
    }
    Ok(())
}

/// One line of what was done or found, under its subject.
fn say(subject: &str, line: &str) {
    println!("✓ {subject:<9} {line}");
}

/// The target of the app's that `server` names - the only one when it names
/// none - or `None` when there is none to take.
fn pick(targets: &[Target], server: Option<&str>, name: &str) -> Result<Option<Target>, Box<dyn Error>> {
    let mine: Vec<&Target> = targets.iter().filter(|t| t.app == name && server.is_none_or(|s| t.server == s)).collect();
    match mine.as_slice() {
        [] => Ok(None),
        [one] => Ok(Some((*one).clone())),
        many => {
            let named: Vec<String> = many.iter().map(|t| format!("{} (on {})", t.name, t.server)).collect();
            let how = if server.is_some() {
                "turnout names it"
            } else {
                "name the server with --server"
            };
            Err(format!("turnout has {} targets for {name}: {} - {how}", many.len(), named.join(", ")).into())
        }
    }
}

/// The target the stand is deployed through: the one turnout has, or one
/// made now on `server`, with its path, its address and its check brought in
/// line with turnout.json.
fn route(described: &Described, stand: &Stand, server: Option<&str>, dir: Option<&str>) -> Result<Target, Box<dyn Error>> {
    let name = &described.name;
    let targets = turnout::targets().map_err(told)?;
    let servers = turnout::servers().map_err(told)?;
    let found = pick(&targets, server, name)?;
    let (target, made) = match found {
        Some(target) => (target, false),
        None => {
            let Some(server) = server else {
                let known: Vec<&str> = servers.iter().map(|s| s.name.as_str()).collect();
                let known = if known.is_empty() {
                    "it knows none yet - `turnout server add` adds one".to_string()
                } else {
                    format!("it knows {}", known.join(", "))
                };
                return Err(format!("turnout has no stand for {name} yet: `lyrn stand up --server NAME` makes one - {known}").into());
            };
            (make_target(described, stand, server, dir, &servers)?, true)
        }
    };
    if !made && dir.is_some_and(|d| d != target.dir) {
        return Err(format!(
            "the stand is in {} already; --dir only says where the first one goes - `turnout path edit {}` moves it",
            target.dir, target.path
        )
        .into());
    }
    let on = servers.iter().find(|s| s.name == target.server);
    if let Some(shell) = on.and_then(|s| s.shell.as_deref()).filter(|shell| *shell != "posix") {
        return Err(format!("{} runs a {shell} shell, and a stand's commands are POSIX shell", target.server).into());
    }
    let host = on.and_then(turnout::Server::host);
    in_line(&target, stand, host.as_deref())?;
    // Read again: the address and the check are the ones just written.
    let target = turnout::targets()
        .map_err(told)?
        .into_iter()
        .find(|t| t.name == target.name)
        .ok_or_else(|| format!("turnout lost the target {} it was just given", target.name))?;
    if made {
        say("target", &format!("{} made: {} on {}, in {}", target.name, name, target.server, target.dir));
    } else {
        say("target", &format!("{}: {} on {}, in {}", target.name, name, target.server, target.dir));
    }
    Ok(target)
}

/// A target for the app on `server`, through a path of its own.
fn make_target(described: &Described, stand: &Stand, server: &str, dir: Option<&str>, servers: &[turnout::Server]) -> Result<Target, Box<dyn Error>> {
    let name = &described.name;
    let Some(found) = servers.iter().find(|s| s.name == server) else {
        let known: Vec<&str> = servers.iter().map(|s| s.name.as_str()).collect();
        return Err(format!(
            "turnout has no server `{server}` - it knows {}; `turnout server add {server}` adds it",
            if known.is_empty() { "none".to_string() } else { known.join(", ") }
        )
        .into());
    };
    let host = found
        .host()
        .ok_or_else(|| format!("turnout knows no host for `{server}`: neither its URL nor its SSH host says one"))?;
    let dir = dir.map(str::to_string).unwrap_or_else(|| format!("/srv/{name}"));
    if !dir.starts_with('/') || dir.contains('\'') {
        return Err(format!("`{dir}` is not a directory a stand can live in: it has to be absolute, and without a quote").into());
    }
    let place = format!("{name}-stand");
    let restart = Described::on_stand(stand, &stand.restart);
    let places = turnout::places().map_err(told)?;
    match places.iter().find(|p| p.name == place) {
        Some(existing) if existing.dir != dir => {
            return Err(format!(
                "turnout has a path `{place}` in {} already - `--dir {}` deploys there",
                existing.dir, existing.dir
            )
            .into());
        }
        Some(_) => {}
        None => {
            turnout::ask(&["path", "add", &place, "--dir", &dir, "--restart", &restart]).map_err(told)?;
        }
    }
    let url = format!("http://{host}:{}", stand.port);
    let target_name = format!("{name}-{server}");
    let mut args: Vec<&str> = vec![
        "target",
        "add",
        target_name.as_str(),
        "--app",
        name.as_str(),
        "--server",
        server,
        "--path",
        place.as_str(),
        "--url",
        url.as_str(),
    ];
    if let Some(health) = &stand.health {
        args.extend(["--health", health.as_str()]);
    }
    turnout::ask(&args).map_err(told)?;
    // turnout gives a taken name a suffix rather than overwriting; the
    // target is the app's on that server and path, whatever it was called.
    turnout::targets()
        .map_err(told)?
        .into_iter()
        .find(|t| t.app == *name && t.server == server && t.path == place)
        .ok_or_else(|| format!("turnout made no target for {name} on {server}").into())
}

/// The path's restart, the target's address and its check, as turnout.json
/// says them. An address or a check somebody chose by hand - a proxy's host
/// name, a command - is theirs, and stays.
fn in_line(target: &Target, stand: &Stand, host: Option<&str>) -> Result<(), Box<dyn Error>> {
    let restart = Described::on_stand(stand, &stand.restart);
    let places = turnout::places().map_err(told)?;
    if let Some(place) = places.iter().find(|p| p.name == target.path)
        && place.restart.as_deref() != Some(restart.as_str())
    {
        turnout::ask(&["path", "edit", &place.name, "--restart", &restart]).map_err(told)?;
        say("restart", &format!("{} runs `{restart}` now", place.name));
    }

    let ours = |url: &str| match (turnout::authority(url), host) {
        (Some((at, Some(_))), Some(host)) => at == host,
        _ => false,
    };
    let wanted = host.map(|h| format!("http://{h}:{}", stand.port));
    let stale = match target.url.as_deref() {
        None => true,
        Some(url) => ours(url) && turnout::authority(url).and_then(|(_, port)| port) != Some(stand.port),
    };
    if let (true, Some(wanted)) = (stale, &wanted)
        && target.url.as_deref() != Some(wanted.as_str())
    {
        turnout::ask(&["target", "edit", &target.name, "--url", wanted]).map_err(told)?;
        say("address", &format!("{} answers at {wanted}", target.name));
    }

    if let Some(health) = &stand.health {
        let by_hand = target.health.as_ref().is_some_and(|h| h.command.is_some());
        let asked = target.health.as_ref().and_then(|h| h.url.as_deref());
        if !by_hand && asked != Some(health.as_str()) {
            turnout::ask(&["target", "edit", &target.name, "--health", health]).map_err(told)?;
            say("health", &format!("{} is asked {health} after every deploy", target.name));
        }
    }
    Ok(())
}

/// The stand's own files - a database password - made on the stand the first
/// time, and never sent from here: a secret that never leaves the stand is
/// one no upload, log or repository can carry off.
fn make_secrets(target: &Target, stand: &Stand) -> Result<(), Box<dyn Error>> {
    if stand.secrets.is_empty() {
        return Ok(());
    }
    let mut script = format!("set -e; mkdir -p '{}'; cd '{}'", target.dir, target.dir);
    for (file, variables) in &stand.secrets {
        if file.contains('\'') || file.starts_with('/') || file.split('/').any(|part| part == "..") {
            return Err(format!("{} names a secret file `{file}` outside the stand's directory", turnout::FILE).into());
        }
        script.push_str(&format!("; mkdir -p \"$(dirname '{file}')\"; touch '{file}'; chmod 600 '{file}'"));
        for variable in variables {
            if !variable.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                return Err(format!("{} names a secret `{variable}` that is not a variable's name", turnout::FILE).into());
            }
            script.push_str(&format!(
                "; grep -q '^{variable}=' '{file}' || {{ printf '{variable}=%s\\n' \"$(od -An -N24 -tx1 /dev/urandom | tr -d ' \\n')\" >> '{file}'; echo 'made {variable} in {file}'; }}"
            ));
        }
    }
    // turnout repeats the command it ran before what went wrong; the script
    // is lyrn's, and the last line is the answer.
    let said = turnout::ask(&["exec", &target.name, "--dir", "/", "--", &script]).map_err(|trouble| match trouble {
        Trouble::Refused(said) => {
            let last = said.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or_default();
            let last = last.trim().strip_prefix("error: ").unwrap_or(last.trim());
            format!("the stand's own files could not be made on {}: {last}", target.server).into()
        }
        missing => told(missing),
    })?;
    let made: BTreeSet<&str> = said.lines().filter_map(|l| l.trim().strip_prefix("made ")).collect();
    for (file, variables) in &stand.secrets {
        let fresh: Vec<&str> = variables
            .iter()
            .map(String::as_str)
            .filter(|v| made.contains(format!("{v} in {file}").as_str()))
            .collect();
        let line = if fresh.is_empty() {
            format!("{file} on the stand holds {}", variables.join(", "))
        } else {
            format!("{file} on the stand holds {}, made there just now", variables.join(", "))
        };
        say("secrets", &line);
    }
    Ok(())
}

/// The target the stand already has: `seed` and `image` work on a stand
/// that is up, and do not make one.
fn standing(name: &str, server: Option<&str>) -> Result<Target, Box<dyn Error>> {
    let targets = turnout::targets().map_err(told)?;
    pick(&targets, server, name)?.ok_or_else(|| {
        let on = server.map(|s| format!(" on {s}")).unwrap_or_default();
        format!("{name} has no stand{on} yet: `lyrn stand up --server NAME` puts it up").into()
    })
}

fn seed(root: &Path, server: Option<&str>) -> Result<(), Box<dyn Error>> {
    let (described, stand) = described(root)?;
    let seed = stand.seed.clone().ok_or_else(|| {
        format!(
            "{} gives the stand no seed - the service form writes one with `--with demo`, whose rows say they are a demonstration",
            turnout::FILE
        )
    })?;
    let target = standing(&described.name, server)?;
    let command = Described::on_stand(&stand, &seed);
    say("seed", &format!("{command}, on {}", target.server));
    let status = turnout::run(&["exec", &target.name, "--", &command]).map_err(told)?;
    if !status.success() {
        return Err(format!("the seed did not run on {}: what it printed above says why", target.server).into());
    }
    let Some(health) = &stand.health else {
        println!("\nSeeded; {} gives no health to ask whether it took.", turnout::FILE);
        return Ok(());
    };
    let address = target
        .url
        .clone()
        .or(target.address.clone())
        .ok_or("turnout knows no address for the stand to ask")?;
    let url = format!("{}{health}", address.trim_end_matches('/'));
    let body = ask_until_it_answers(&url, Duration::from_secs(90))?;
    let demo = serde_json::from_str::<Value>(&body).ok().and_then(|v| v.get("demo").and_then(Value::as_bool));
    match demo {
        Some(true) => say("health", &format!("{url} answers, and says it is a demonstration")),
        _ => say("health", &format!("{url} answers, without saying it is a demonstration")),
    }
    Ok(())
}

/// GET `url` until it answers below 400, or the wait is over; its body.
fn ask_until_it_answers(url: &str, wait: Duration) -> Result<String, Box<dyn Error>> {
    let client = reqwest::blocking::Client::builder().timeout(Duration::from_secs(10)).build()?;
    let start = Instant::now();
    let mut last = String::from("no answer");
    while start.elapsed() < wait {
        match client.get(url).send() {
            Ok(response) if response.status().as_u16() < 400 => return Ok(response.text().unwrap_or_default()),
            Ok(response) => last = format!("it answered {}", response.status().as_u16()),
            Err(e) => last = e.to_string().lines().next().unwrap_or_default().to_string(),
        }
        std::thread::sleep(Duration::from_secs(2));
    }
    Err(format!("{url} did not answer in {}s: {last}", wait.as_secs()).into())
}

fn image(root: &Path, server: Option<&str>) -> Result<(), Box<dyn Error>> {
    let (described, stand) = described(root)?;
    let image = stand
        .image
        .clone()
        .ok_or_else(|| format!("{} says nothing builds the stand's image: its `stand.image` is that command", turnout::FILE))?;
    let target = standing(&described.name, server)?;
    // A directory beside the stand's: the build gets the same machine and
    // the same image cache, and the stand keeps running what it runs.
    let place = format!("{}-image", described.name);
    let dir = format!("{}-image", target.dir.trim_end_matches('/'));
    let places = turnout::places().map_err(told)?;
    match places.iter().find(|p| p.name == place) {
        Some(existing) if existing.restart.as_deref() == Some(image.as_str()) && existing.dir == dir => {}
        Some(_) => {
            turnout::ask(&["path", "edit", &place, "--dir", &dir, "--restart", &image]).map_err(told)?;
        }
        None => {
            turnout::ask(&["path", "add", &place, "--dir", &dir, "--restart", &image]).map_err(told)?;
        }
    }
    say("image", &format!("built on {} in {dir}, beside the stand", target.server));
    let status = turnout::run(&["deploy", &target.name, "--path", &place]).map_err(told)?;
    if !status.success() {
        return Err(format!(
            "the image did not build on {}: turnout's checklist above says why, and the stand runs what it ran",
            target.server
        )
        .into());
    }
    println!("\nThe image of HEAD builds on {}; the stand still runs what it ran.", target.server);
    Ok(())
}

/// What `git` prints in `root`, trimmed; `None` when it fails.
fn git(root: &Path, args: &[&str]) -> Option<String> {
    let out = Process::new("git").args(args).current_dir(root).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn image_in_ci(root: &Path) -> Result<(), Box<dyn Error>> {
    let workflow = root.join(".github").join("workflows").join(IMAGE_WORKFLOW);
    if !workflow.is_file() {
        return Err(format!("no .github/workflows/{IMAGE_WORKFLOW} here: it is the build in CI - the service form writes it").into());
    }
    let repo = crate::commands::check::product::Product::read(&crate::commands::doctor::Repo::on_disk(root))
        .github
        .ok_or("no GitHub repository is named here: `origin` points at none")?;
    let head = git(root, &["rev-parse", "HEAD"]).ok_or("this is not a repository with a commit")?;
    let branch = git(root, &["rev-parse", "--abbrev-ref", "HEAD"])
        .filter(|b| b != "HEAD")
        .ok_or("HEAD is on no branch: the run is asked for a branch")?;
    let pushed = git(root, &["rev-parse", "@{upstream}"]);
    if pushed.as_deref() != Some(head.as_str()) {
        return Err(format!("{branch} on GitHub is not HEAD: push first - the run builds what the branch is there").into());
    }

    let api = crate::github::Api::new()?;
    if !api.signed_in() {
        return Err("no GitHub token at hand: GH_TOKEN, or `gh auth login`".into());
    }
    let runs = |api: &crate::github::Api| -> Result<Vec<(u64, String, String, String)>, Box<dyn Error>> {
        let found = api.get(&format!(
            "repos/{repo}/actions/workflows/{IMAGE_WORKFLOW}/runs?event=workflow_dispatch&head_sha={head}&per_page=20"
        ))?;
        Ok(found
            .get("workflow_runs")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|run| {
                let text = |key: &str| run.get(key).and_then(Value::as_str).unwrap_or_default().to_string();
                Some((run.get("id")?.as_u64()?, text("status"), text("conclusion"), text("html_url")))
            })
            .collect())
    };
    let before: BTreeSet<u64> = runs(&api)?.into_iter().map(|(id, ..)| id).collect();
    api.send(
        "POST",
        &format!("repos/{repo}/actions/workflows/{IMAGE_WORKFLOW}/dispatches"),
        Some(&serde_json::json!({ "ref": branch })),
    )?;
    let short = &head[..head.len().min(7)];
    say("image", &format!("{IMAGE_WORKFLOW} asked to build {short} on {branch}"));

    let start = Instant::now();
    let mut announced = false;
    loop {
        if start.elapsed() > Duration::from_secs(90 * 60) {
            return Err(format!("{IMAGE_WORKFLOW} did not finish in 90 minutes").into());
        }
        std::thread::sleep(Duration::from_secs(if announced { 20 } else { 3 }));
        let Some((_, status, conclusion, url)) = runs(&api)?.into_iter().find(|(id, ..)| !before.contains(id)) else {
            if start.elapsed() > Duration::from_secs(120) {
                return Err(format!("GitHub started no run of {IMAGE_WORKFLOW} for {short} in two minutes").into());
            }
            continue;
        };
        if !announced {
            say("run", &url);
            announced = true;
        }
        if status != "completed" {
            continue;
        }
        return if conclusion == "success" {
            println!("\nThe image of {short} builds and starts on both architectures: {url}");
            Ok(())
        } else {
            Err(format!("{IMAGE_WORKFLOW} ended {conclusion} for {short}: {url}").into())
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(name: &str, server: &str) -> Target {
        Target {
            name: name.into(),
            app: "demo-svc".into(),
            server: server.into(),
            path: "demo-svc-stand".into(),
            dir: "/srv/demo-svc".into(),
            url: None,
            address: None,
            health: None,
        }
    }

    #[test]
    fn the_only_target_is_taken_and_several_need_a_server() {
        let one = [target("demo-svc-pi", "pi")];
        assert_eq!(pick(&one, None, "demo-svc").unwrap().unwrap().name, "demo-svc-pi");
        assert_eq!(pick(&one, Some("vps"), "demo-svc").unwrap(), None);
        assert_eq!(pick(&one, None, "other").unwrap(), None);

        let two = [target("demo-svc-pi", "pi"), target("demo-svc-vps", "vps")];
        let error = pick(&two, None, "demo-svc").unwrap_err().to_string();
        assert!(
            error.contains("demo-svc-pi (on pi), demo-svc-vps (on vps)") && error.contains("--server"),
            "{error}"
        );
        assert_eq!(pick(&two, Some("vps"), "demo-svc").unwrap().unwrap().name, "demo-svc-vps");
    }
}
