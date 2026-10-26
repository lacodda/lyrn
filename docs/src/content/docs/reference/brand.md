---
title: lyrn brand
description: A new product's mark held to the line's - its code, its colour or pair, and the two together against marks that already mean something.
---

```console
$ lyrn brand pick [name] [--code <xx>] [--colour <#rrggbb> [--to <#rrggbb>]]
```

Every product of the line wears a mark: a two-letter code on a hexagon in the
product's colour - one hue, or a pair drawn as one gradient. The owner chooses
code and colour on a board, by eye. `lyrn brand pick` holds the choice to what
an eye cannot see at once: a code another product has, a hue too close to a
neighbour's, a pair already taken in that direction, and a code on a colour that
is somebody else's mark - `mi` on orange is Xiaomi, which the line found out
only after the assets were laid out.

Without a code it lists the codes the line's rule makes of the name - its
first letter and a consonant after it, or the first letter of each word:

```console
$ lyrn brand pick nova-sync
lyrn brand pick: nova-sync, against the line's 20 marks

code
  ✓ ns  free
  ✓ nv  free
  ✓ nn  free
  ✓ nc  free
  ! no  reads as a word

free for a single colour: 126° #45D353 (24° clear), 308° #D345C0 (19° clear), 87° #94D345 (15° clear)
```

With a code and a colour it says what each is, and what they are together:

```console
$ lyrn brand pick midda --code mi --colour '#D9704A'
lyrn brand pick: midda, against the line's 20 marks

code
  ✓ mi  free

colour #D9704A
  ✗ 0° from furca #D9704A (16°) and 0° from scheda #D9704A (16°): a single colour needs 12° - this one is fit for the second colour of a pair

code on colour
  ✗ mi on #D9704A (16°) is Xiaomi: mi in white on orange
```

It exits with 1 when the code or the colour chosen is taken, so the board's
choice can be checked before it is written into the registry.

## What it holds the choice to

- **The code** is two lowercase letters no other product has. One that reads
  as a word or as something else first - `hi`, `sd`, `rg` - warns.
- **A single colour** stands at least 12° of hue from every colour of the line,
  the colours of pairs included; closer, it is fit only as the second colour of
  a pair. A grey has no hue: graphite is the umbrella mark's.
- **A pair** may take any colour of the palette. Only the same two colours in
  the same direction are taken - the reverse of a pair is free.
- **The code on the colour** is checked against marks that are two letters on
  a colour already: the ones the line's board caught by eye - Xiaomi, EF,
  Red Hat, Kickstarter, Deutsche Welle, LinkedIn, Lyft - and Adobe's app icons,
  which are the line's own pattern: two letters on a dark tile, one colour per
  product. A resemblance the owner has accepted before, as `ln` on indigo next
  to LinkedIn's `in`, warns rather than fails.

The free hues are the middle of every arc of the colour wheel that leaves a
single colour its 12° on both sides, widest first, each with a colour of that
hue at the saturation and lightness the line's colours share - something to
try on the board, not a decision.

## The registry

The marks are the ones [dowel](https://lacodda.github.io/dowel) publishes: lyrn
carries a copy of each product's code and colours, read from the S level of
its mark - the filled tile, which draws exactly those. The copy moves with the
dowel-ui the templates pin, and each product's first colour is held to the
accent dowel gives it, so the two cannot disagree.
