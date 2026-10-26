---
title: lyrn assets
description: Lay the mark out by the line's canon and draw every raster of it - the .ico, the favicon, touch icons, a manifest's icons, the social preview - from its masters.
---

```console
$ lyrn assets [path] [--from <dir>] [--mark <name>]
```

A product's mark is four SVG masters in `assets/`: `logo.svg` (L),
`logo-m.svg` (M), `logo-s.svg` (S) and `banner.svg`. Everything else a
repository carries of it is drawn from them and never edited by hand, and
`lyrn assets` draws it - each size from the level that reads at it: S at 27px
and below, M from 28 to 63, L from 64 up.

```console
$ lyrn assets --from ~/brand/svg
lyrn assets: ~/lyrn, lyrn

wrote      assets/apple-touch-icon.png       L at 180px
wrote      assets/favicon-32.png             S at 32px, the favicon's fallback
wrote      assets/icon.ico                   every size from its level: L 256-64, M 48-32, S 24-16, the largest first
wrote      assets/logo-512.png               L at 512px, for the places outside
wrote      assets/social-preview.png         the banner's artwork on the plate, 1280x640
wrote      docs/public/apple-touch-icon.png  L at 180px
unchanged  assets/logo.svg                   lyrn-L.svg, the master of L
unchanged  assets/logo-m.svg                 lyrn-M.svg, the master of M
unchanged  assets/logo-s.svg                 lyrn-S.svg, the master of S
unchanged  assets/banner.svg                 lyrn-banner.svg, the README's banner
unchanged  docs/public/favicon.svg           a copy of the S master: a favicon reads in a tab
unchanged  docs/src/assets/logo.svg          a copy of the L master: a site's header

assets/social-preview.png changed: upload it by hand at https://github.com/lacodda/lyrn/settings → Social preview - GitHub has no API for it
```

## What it draws, and where

| File | Drawn from |
| --- | --- |
| `assets/favicon-32.png` | S at 32px - a favicon is drawn into 16px of tab whatever its size, so it is the one exception to the rule |
| `assets/apple-touch-icon.png` | L at 180px |
| `assets/logo-512.png` | L at 512px |
| `assets/social-preview.png` | the banner, 1280x640 |
| the `.ico` the form's build reads | 256, 128 and 64 from L, 48 and 32 from M, 24 and 16 from S, the largest first |
| `src-tauri/icons/icon.png` (desktop) | L at 512px |

The `.ico` goes where the form's build reads it: `assets/icon.ico` for a cli,
`crates/<name>/assets/icon.ico` for a workspace - a published crate carries
only what is under its own directory - and `src-tauri/icons/icon.ico` for a
desktop app. The largest image comes first: Windows picks by size and ignores
the order, but Tauri takes the first image as the window's icon.

Beyond that canon, every copy the repository already keeps is drawn again where
it is: any other `.ico`, a site's header (`src/assets/logo.svg`, the L master)
and favicon (`favicon.svg`, the S master), every `apple-touch-icon.png`, the
square PNGs in `src-tauri/icons/`, and the icons a web app's
`manifest.webmanifest` lists - all but a maskable one, which is drawn with room
for the mask the mark has not got. A copy is kept where it is, not created
where nothing reads it. A file that already holds what would be written is left
alone.

## Taking the masters in

`--from` takes the four masters in first, from the directory the mark's
generator writes them to: `<mark>-L.svg`, `<mark>-M.svg`, `<mark>-S.svg` and
`<mark>-banner.svg`. The mark is the project's name unless `--mark` names
another. A mark with no banner - the line's umbrella mark has none - is taken
without one, and the social preview waits for it.

A project still wearing the line's umbrella mark says `mark = "placeholder"` in
`lyrn.toml`; taking in a mark of its own sets it to `chosen`, which is what
[`lyrn doctor`](/reference/doctor/#mark) reads.

## The social preview

GitHub shows it wherever the repository is linked, and crops its edges in some
of those places. The banner's artwork - the tile, the name, the line, the
underline, wherever the product's words make it end - is found by drawing the
banner on the plate, then drawn again fitted into 1040x460 and centred on the
1280x640 plate. A crop at a fixed width that suits one product cuts another's
underline short.

GitHub has no API for it: upload `assets/social-preview.png` by hand, under
Social preview in the repository's settings.

## The font

The masters set their code and their banner's line in Cascadia Code. A
rasterizer that asks the machine for that font draws another picture wherever
another monospace stands in, so lyrn carries the font - Cascadia Code 2407.24,
unmodified, under the SIL Open Font License 1.1, which travels with every
release as `LICENSE-CascadiaCode` - and asks the machine for nothing. The same
masters give the same pixels everywhere. A character the font has no glyph for
is refused, naming the text that holds it, rather than drawn as an empty box.

## And the doctor

[`lyrn doctor`](/reference/doctor/#assets) holds a repository to the same rule
by reading the files, and its `--fix` draws a wrong `.ico` or touch icon again
with this code. An exporter of the repository's own - `docs/export-assets.mjs`,
as the line once wrote them - is a second drawing of the rule; the doctor warns
about it, and it can go.
