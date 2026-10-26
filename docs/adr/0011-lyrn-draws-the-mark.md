# ADR 0011: lyrn draws the mark, with the line's font inside it

- Status: accepted
- Date: 2026-10-26

## Context

A product's mark is four SVG masters - three levels, L, M and S, and a banner -
and a repository carries a dozen rasters drawn from them: the `.ico` the build
embeds, a favicon, touch icons, the icons a web app's manifest lists, a social
preview. Until 2.15 they were drawn by `docs/export-assets.mjs`, which the
documentation site brought along, through sharp and librsvg; the placeholder
icons the forms ship were drawn by a Python script of their own.

Both were second drawings of one rule, and the line paid for each copy. Nine
products of fifteen wrote their own exporter and put a wrong level somewhere -
a whole `.ico` cut from the S tile, a touch icon from S at 180px. And the
masters set their code and their banner's line in Cascadia Code by name: a
rasterizer that asks the machine for that font draws a different picture on
every machine. The social preview's crop, worked out on one machine, cut the
end of lyrid's underline on another.

## Decision

`lyrn assets` draws every raster, with resvg, from the masters in `assets/`.
`lyrn assets --from <directory>` takes the masters in first, from the mark's
generator. The level rule, the order of the `.ico`, the favicon's exception and
the social preview's crop live in one place, `src/mark/`, and the doctor's
`--fix` draws a wrong `.ico` again with the same code.

lyrn carries the font: Cascadia Code 2407.24, the variable font as Microsoft
released it, unmodified, under the SIL Open Font License 1.1. The masters ask
for it by its name, and its Reserved Font Name allows that only of an
unmodified copy, so it is not cut down to the glyphs the line uses; it costs
the binary about 720 KB. Every family a master may name, generic ones included,
resolves to it, and nothing is asked of the machine. A character the font has
no glyph for is refused with the text that holds it, rather than drawn as an
empty box.

The placeholder rasters the forms ship are the umbrella mark drawn by the same
code; a test holds the bytes in `src/templates/` to that drawing, pixel by
pixel within two steps, and `LYRN_BLESS=1` draws them again.

## Consequences

- The same masters give the same pixels on every machine and in CI.
- The documentation site no longer brings an exporter, and the Python script is
  gone. sharp stays in the site's packages: it is Astro's image service.
- The font's licence travels with every release archive, as
  `LICENSE-CascadiaCode`, and with the crate, in `src/mark/font/`.
- A new version of resvg may move pixels: the placeholder test says so, and
  `LYRN_BLESS=1` takes the new drawing.
