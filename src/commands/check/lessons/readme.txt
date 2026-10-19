crates.io and npm show a README of the package's own, rendered away from the
repository. One README serves them all: the root one, which a crate names in
`readme` and an npm wrapper copies in when it is packed.

A second README beside a published package is a shopfront nobody updates when
the first one changes, so the rule fails a README file next to a published
package of the product, and a crate whose `readme` names anything but the root
README.

It also fails every link and image in the root README written relative to the
repository: `assets/logo.svg` resolves on GitHub and nowhere else. Links to
`http`, `https`, `mailto` and `#anchors` pass; links in code are not links.
