---
title: lyrn stand
description: Put a service on its stand through turnout, fill it for a demonstration, and build its image on the stand's own machine or in CI.
---

```console
$ lyrn stand up [--server <server>] [--dir <dir>] [path]
$ lyrn stand seed [--server <server>] [path]
$ lyrn stand image [--server <server> | --ci] [path]
```

A stand is where a service runs for real: its own machine, its own database,
its own address. [turnout](https://lacodda.github.io/turnout/) puts it there -
it builds, uploads, restarts, asks the stand's health and rolls a deploy that
fails the check back. `lyrn stand` tells turnout what the stand is, from the
project's `turnout.json`, and then lets it work.

## What the stand is: turnout.json

The [service form](/reference/forms/#service) writes it, and [`lyrn new`](/reference/new/#turnout)
registers it with turnout:

```json
{
  "name": "demo-svc",
  "commands": {
    "build": "sh tools/stand/stage.sh target/stand",
    "dev": "cargo run",
    "lint": "cargo clippy --all-targets -- -D warnings",
    "test": "cargo test"
  },
  "dist": "target/stand",
  "stand": {
    "port": 8100,
    "restart": "docker compose -f deploy/compose.yml up -d --build --remove-orphans",
    "health": "/api/health",
    "image": "docker build --tag demo-svc-server:candidate .",
    "seed": "DEMO_SVC_DEMO=true docker compose -f deploy/compose.yml up -d --force-recreate --no-deps server",
    "secrets": {
      "deploy/.env": ["POSTGRES_PASSWORD"]
    }
  }
}
```

| Key | What it says |
| --- | --- |
| `name` | The app's name in turnout's catalog |
| `commands` | turnout's roles and any other command; `build` is what a deploy runs before it uploads |
| `dist` | What a deploy uploads, relative to the project |
| `stand.port` | The port the stand answers on; every command on the stand is handed it as `STAND_PORT` |
| `stand.restart` | What puts an upload to work, in the directory it landed in |
| `stand.health` | The path that says the stand works, asked after every deploy |
| `stand.image` | What builds the stand's image without replacing what runs |
| `stand.seed` | What fills the stand with made-up data; only with `--with demo` |
| `stand.secrets` | Files of the stand's own and the variables each holds - made there, never uploaded |

What is the project's lives here, in the repository, the same for everyone who
clones it. What is the machine's - which server, who logs in, where the files
land - lives in turnout's catalog and nowhere else. A key the file does not
have is refused rather than ignored: a misspelt `helth` is a stand nobody asks.

### The port

A new service is given the first port of `8100-8199` that no stand the machine
knows holds: every address in turnout's catalog, and every port the line's
projects in rigger's record have been given. Two services on one server are
never handed the same one. The compose file publishes `${STAND_PORT:-8080}`, so
the number lives in turnout.json alone - 8080 on a laptop, the stand's own on
the stand.

## up

```console
$ lyrn stand up --server home
✓ app       demo-svc is in turnout's catalog now
✓ target    demo-svc-home made: demo-svc on home, in /srv/demo-svc
✓ binding   demo-svc uses home now, where a release deploys it
✓ secrets   deploy/.env on the stand holds POSTGRES_PASSWORD, made there just now
✓ deploy    turnout deploy demo-svc-home
...
demo-svc is up on home: http://home.example:8100
```

1. **The app.** Added to turnout's catalog as turnout.json describes it, or the
   entry already there brought in line with the file. It has no gateway port: a
   service talks to its own stand, and a port would have turnout write a dotenv
   file into a repository with no use for one.
2. **The target.** The first time, `--server` names the server - one turnout
   already knows - and a target is made on it through a path of its own,
   `<name>-stand`, in `--dir` (default `/srv/<name>`). After that the target is
   found; `--server` picks one when there are several. Its path runs
   `stand.restart` with the port, it answers at the server's host on
   `stand.port`, and it is asked `stand.health` after every deploy. An address
   or a check set by hand - a proxy's host name, a command - is left as it is.
3. **The binding.** An app bound to no server is bound to the stand's, which is
   where a release's `[post.deploy]` deploys it: `furca release` runs `turnout
   deploy` with no target.
4. **The secrets.** Each variable of `stand.secrets` missing from its file is
   made on the stand from its own random source. The password never exists on
   this machine, so no upload, log or repository can carry it off; the deploy
   never touches the file again.
5. **The deploy.** `turnout deploy`, in this terminal: the build stages the
   repository at HEAD into `target/stand` - tracked files only, so `target/`
   and a `.env` cannot travel - the upload sends it, and the restart builds the
   image **on the stand's machine**. A Raspberry Pi builds for arm64 itself;
   nothing is cross-compiled. A deploy that fails its check is rolled back.

Run it again after any change: everything already so is said so, and only
what turnout.json says otherwise is changed.

## seed

```console
$ lyrn stand seed
✓ seed      STAND_PORT=8100 DEMO_SVC_DEMO=true docker compose -f deploy/compose.yml up -d --force-recreate --no-deps server, on home
✓ health    http://home.example:8100/api/health answers, and says it is a demonstration
```

Runs `stand.seed` on the stand, then asks its health until it answers. The
service form's seed starts the server once with its demo flag on: an empty
database is filled with fixed rows and marked as a demonstration, and one
holding real data refuses to start with the flag, so a seed can never mix
invented rows into real ones. The mark lives in the data, so the stand stays a
demonstration after the next deploy, without the flag. A stand that is not up
yet is refused with the way to put it up.

## image

```console
$ lyrn stand image
✓ image     built on home in /srv/demo-svc-image, beside the stand
...
The image of HEAD builds on home; the stand still runs what it ran.
```

Builds the stand's image from HEAD **on the stand's own machine**, through a
path beside the stand's (`<name>-image`), with `stand.image`. The stand keeps
running what it runs, and its health is not asked: this is the build alone,
for the commit about to be released, on the architecture it will run on.

### In CI

```console
$ lyrn stand image --ci
✓ image     image.yml asked to build 3f9c1e0 on main
✓ run       https://github.com/owner/demo-svc/actions/runs/123456789
The image of 3f9c1e0 builds and starts on both architectures: https://github.com/owner/demo-svc/actions/runs/123456789
```

For a stand out of reach: the service form's `.github/workflows/image.yml` runs
on `workflow_dispatch` only - an image build is minutes of Rust the CI of every
push already compiles - and builds and starts the stand on `amd64` and `arm64`
runners, then asks its health. `--ci` asks GitHub for a run on the branch HEAD
is on, waits for it, and says how it ended. HEAD has to be pushed: the run
builds what the branch is on GitHub.

## Options

| Option | What it does |
| --- | --- |
| `[path]` | The repository; defaults to the one the current directory is in |
| `--server <server>` | The server the stand is on, as turnout names it; needed by `up` the first time, and to pick one of several |
| `--dir <dir>` | Where on the server the stand lives, the first time; defaults to `/srv/<name>` |
| `--ci` | With `image`: build in CI instead, on both architectures, for the commit HEAD is |
