---
title: lyrn add
description: Copy items of dowel's registry into a project, from the dowel-ui it pins, with every part they are made of.
---

```console
$ lyrn add <item>... [--path <path>]
```

A product of the line takes its primitives and screens from dowel's registry
as copies: a Button, a PasswordField, a LoginScreen. `lyrn add` writes them
into the project with everything they are made of, from the registry inside the
`dowel-ui` the project installed:

```console
$ lyrn add login-screen
From dowel-ui 1.0.0, in .:
  kept   src/components/ui/alert.tsx
  kept   src/components/ui/button.tsx
  added  src/components/ui/field.tsx
  added  src/components/ui/input.tsx
  added  src/components/ui/panel.tsx
  added  src/components/ui/password-field.tsx
  added  src/components/blocks/login-screen.tsx
```

## Why from `node_modules`

`npx shadcn add` with dowel's address reads the registry on dowel's site,
which is the newest dowel there is - not necessarily the one the project
installed. A copy of a newer Button in a project on an older dowel-ui is the
drift the project's own `pnpm registry` refuses, so the first thing such an add
does is fail the lint. lyrn reads `node_modules/dowel-ui/dist/registry.json`
instead: the same file the check holds the copies to, so what lands is what
is checked. The pin is the project's own - no flag says which dowel - and a
project not installed yet is told to `pnpm install` first.

## What lands where

The places are the project's `components.json`: a primitive goes to
`src/components/ui/`, a block to `src/components/blocks/`. Every part an item
is made of comes with it, each once, before what needs it - a list beside its
detail brings the frame, which brings a scroll area. Nothing is written until
every item has been found: a name the registry has no copy of, or the theme
and the prose styles, which come with the package itself, refuse the whole
add.

What is there already is decided by whose it is:

| Found | Primitive (`components/ui`) | Block (`components/blocks`) |
| --- | --- | --- |
| nothing | added | added |
| the registry's text | kept | kept |
| something else | taken again - it is dowel's, and `pnpm registry` holds it to the registry | kept - a block is the product's once it lands, and is never written over |

A copy that imports a package `package.json` does not list - `@base-ui/react`,
`@tauri-apps/api` for the window frame - is followed by `pnpm add` for it, or
by the command to run where pnpm cannot.

The project is the directory holding `components.json`: the one given, its
`frontend/` as a service keeps it, or the nearest above, so `lyrn add` works
from anywhere inside the frontend. A project made with
[`lyrn new --with block:<name>`](/reference/forms/#blocks) starts with the same
copies, taken from inside lyrn.

## Options

| Option | What it does |
| --- | --- |
| `<item>...` | Items of dowel's registry, by name: `password-field`, `login-screen`, `product-switcher` |
| `--path <path>` | The project; defaults to the one the current directory is in |
