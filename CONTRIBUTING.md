# Contributing to lyrn

## The gate

```console
$ cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test
```

The same commands CI runs on Linux, macOS and Windows, so a green terminal
means a green pull request. `rust-version` in `Cargo.toml` is a promise to
anyone running `cargo install`, and a CI job builds on exactly that version.

The tests are only half of it. A template that does not build is not a
template, so CI also generates every form - every add-on of the cli, the spa's
add-ons alone and all together - and runs the gate each generated project
ships with, on the platforms it claims. Those jobs are in
`.github/workflows/ci.yml`; a change to a form is not done until they are
green.

## The trees

`tests/trees/` holds what `lyrn new --dry-run` prints for every form and every
combination of its add-ons. A file that moved, vanished or arrived with an
unrelated add-on still builds, and nothing else in the gate would notice. After
a deliberate change:

```console
$ LYRN_BLESS=1 cargo test --test trees
```

then read the diff of `tests/trees/` before committing it.

## Where things live

| Path | What lives there |
| --- | --- |
| `src/cli.rs` | The command surface: every flag and subcommand |
| `src/commands/` | `new`, `init`, `adopt`, `template` |
| `src/templates/` | The built-in forms: one module and one directory of template files each |
| `src/templates/community.rs` | What every repository a form starts carries, written once |
| `src/templates/dowel/` | Copies taken from a published dowel-ui: primitives, accents, marks |
| `src/template/` | Templates from outside: a directory, a GitHub tag, cargo-generate |
| `src/generate.rs`, `src/render.rs` | From a template and a context to files on disk |
| `tests/` | The binary run as a user runs it, and the trees |
| `tools/vendor-dowel.mjs` | Takes the dowel copies from a published dowel-ui |
| `tools/render-placeholder-icon.py` | Draws the placeholder icons the forms ship |
| `docs/` | The documentation site; decisions in `docs/adr/` |

A form is a directory of files plus a list in its module saying which file
goes where and with which add-on. Text files are rendered - `{{ name }}` and
the rest - and a file that has to keep its braces is listed as `verbatim` in
the form's `template.toml`. What one form borrows from another it takes by
rule rather than by copying the file (ADR 0005).

The dowel copies are never edited by hand: `node tools/vendor-dowel.mjs
<version>` takes them from that dowel-ui, and the generated project's own gate
compares them with the dowel-ui it installed.

## Commits

[Conventional Commits](https://www.conventionalcommits.org/), in English:
`feat:`, `fix:`, `docs:`, `chore:` and the rest. The changelog is generated
from them (`cliff.toml`), so a commit message is the line the release notes
will carry.

## Decisions

An architectural decision gets a record in [`docs/adr/`](docs/adr/README.md):
one file, numbered, newest last. A decision that replaces an earlier one says
so, and the earlier one is marked superseded rather than deleted.

## Dependencies

Updated by hand, at most once a week, in a commit of their own - Dependabot is
off on purpose ([ADR 0010](docs/adr/0010-dependabot-is-off.md)). The audit
workflow fails on a published advisory, and on a dependency whose license is
not on the accepted list in `deny.toml`.

## Assistant files

Instructions for coding assistants - `CLAUDE.md`, `AGENTS.md`, `.claude/`,
`.cursor/` and the like - stay on the machine they were written for, and
`.gitignore` keeps them out of the repository. What anyone needs to know about
the project, human or agent, is in this file and in [`llms.txt`](llms.txt).

## Conduct and security

Everyone taking part follows the [Code of Conduct](CODE_OF_CONDUCT.md). A
vulnerability is reported privately, as [SECURITY.md](SECURITY.md) describes -
never in a public issue.

## License

By contributing, you agree that your contributions are licensed under the
project's [MIT license](LICENSE).
