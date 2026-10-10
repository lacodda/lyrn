A README is the shopfront: what the product is, what it looks like at work,
how to install it. The reference lives on the documentation site, building it
in CONTRIBUTING.md, the history in the changelog, the plan nowhere public.

The lesson: a README of a hundred lines looked short and still held three
twenty-line sections and a list item of twelve sentences - all of it already
on the docs site, so the README was a second copy going stale. Counting lines
never showed it. So the rule measures blocks: the longest paragraph, quote or
list item in words, with code and tables left out. About 90 words is a page of
the docs site: past 100 the block fails, between 91 and 100 it is worth a look.

It also fails a section a shopfront does not have (Roadmap, Building,
Development, Layout, Principles, Design notes), a `# title` under a banner that
already carries the name, and a README past 300 lines. It warns when there is
no one-sentence promise as a `>` quote before the first section, and when the
Status section is more than one paragraph of two to four lines (60 words).

Move first, then cut: put the text on the docs site, link to it, and only then
remove it from the README.
