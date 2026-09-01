---
title: lyrn forms
description: The shapes a generated project can take.
---

```console
$ lyrn forms
spa      Single-page app: Vite, React, TypeScript, Tailwind, dowel
```

A form is the shape of the repository, not a choice of framework. The line runs
one stack; a form decides what kind of thing is being built with it.

## spa

A single-page application:

| | |
| --- | --- |
| Build | Vite 8 |
| UI | React 19, TypeScript |
| Styling | Tailwind 4 and the dowel theme |
| Tests | Vitest with jsdom, Testing Library |
| Lint | eslint, typescript-eslint, `dowel/no-raw-color` |

## Coming in 2.x

`cli` (Rust, clap), `desktop` (Tauri), `service` (axum with an SPA), `lib`,
and the workspace and plugin shapes the line already uses.
