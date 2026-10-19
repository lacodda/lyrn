---
title: lyrn versions
description: Where the version is written, and where it went - the manifests, the lockfile and the changelog beside the tag, GitHub's latest release and the registries.
---

```console
$ lyrn versions [path]
```

One screen for the question every release ends with: did it all move? The
files that write the version - the product's manifests, an npm wrapper's pin,
Cargo.lock, the newest section of the changelog - and the places the release
went - the tag behind the commit, GitHub's latest release, crates.io and npm.

```console
$ lyrn versions
lyrn versions: ~/lyrn, lyrn 2.12.0

Written
  2.12.0   Cargo.toml
  2.12.0   npm/package.json
  v2.12.0  npm/package.json (lyrn.binary)
  2.12.0   Cargo.lock (lyrn)
  2.12.0   CHANGELOG.md, the newest section, 2026-10-13

Released
  v2.12.0  tagged 2026-10-13 on 4f5027b, 1 commit since
  v2.12.0  GitHub's latest release of lacodda/lyrn
  2.12.0   crates.io: lyrn
  2.12.0   npm: lyrn

Every file says 2.12.0.
```

The manifests are found the way [`lyrn check versions`](/reference/check/#versions)
finds them, by the standard's layout. A registry is asked about each package of
the product it would publish: crates.io through its sparse index - the files
cargo reads - and npm through its registry; GitHub's latest release is read
from the redirect of `/releases/latest`. None of them spends a quota. A place
that cannot be asked says why on its own line, and the rest of the screen
stands.

`lyrn versions` says; it holds nothing to a rule and exits with 0. The gate is
`lyrn check`.
