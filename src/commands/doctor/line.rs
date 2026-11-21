//! `line`: the line's registry and the repository say the same about the
//! product - its colour, and where its documentation lives.
//!
//! The registry is what `rigger export --line` publishes from the record, and
//! everything that draws the line as a whole reads it: dowel's product
//! switcher and accents, the marks on a README, the list of products on a
//! site. A product whose `lyrn.toml` says one colour while the record says
//! another has buttons in one colour and a tile in the switcher in the other,
//! and neither file looks wrong on its own. Each pair is compared only where
//! both sides say something: a product recorded before its mark was drawn is
//! not wrong for it.
//!
//! The registry's shape of a product is not compared with its form. The form
//! is how the repository is built, and the shape is what the product is to
//! the person using it: a service with a web UI is a web product, and a
//! window drawn by a Rust binary on the cli form is a desktop one. Holding
//! one to the other was wrong for five products of the line the first time it
//! ran.

use super::{Finding, Outcome, Repo};
use crate::commands::check::site::Site;
use crate::rigger::Product;

/// What the registry says of the repository, as `doctor` read it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Listed {
    /// The registry was not read, and why: rigger is not installed here,
    /// or it refused.
    Unread(String),
    /// The registry has no product of this name.
    Absent(String),
    Product(Product),
}

impl Listed {
    /// The registry's entry for `name`, out of what was read.
    pub fn of(registry: &Result<Vec<Product>, String>, name: &str) -> Listed {
        match registry {
            Err(reason) => Listed::Unread(reason.clone()),
            Ok(products) => products
                .iter()
                .find(|p| p.name == name)
                .cloned()
                .map_or_else(|| Listed::Absent(name.to_string()), Listed::Product),
        }
    }
}

pub fn check(repo: &Repo) -> Outcome {
    let product = match &repo.listed {
        Listed::Unread(reason) => return Outcome::Skip(format!("the line's registry was not read - {reason}")),
        Listed::Absent(name) => return Outcome::Skip(format!("the line's registry has no {name}")),
        Listed::Product(product) => product,
    };
    let Ok(facts) = &repo.facts else {
        return Outcome::Skip("its manifests do not parse - `standard` says which".to_string());
    };
    let name = &product.name;
    let mut compared = Vec::new();
    let mut findings = Vec::new();

    if let (Some(listed), Some(ours)) = (&product.accent, &facts.accent) {
        compared.push("colour");
        if !listed.eq_ignore_ascii_case(ours) {
            findings.push(
                Finding::fail(format!("the registry gives {name} {listed}, and lyrn.toml's accent is {ours}")).fix(format!(
                    "one colour is the product's: put it in [project] accent, or in the record with `rigger project mark {name} --accent <hex>`"
                )),
            );
        }
    }

    if let (Some(listed), Some(site)) = (&product.docs, Site::read(repo).and_then(|s| s.address())) {
        compared.push("documentation");
        if normal(listed) != normal(&site) {
            findings.push(
                Finding::fail(format!("the registry sends readers of {name} to {listed}, and the site is built for {site}"))
                    .fix(format!("`rigger project mark {name} --docs {site}`, or the site's `site` and `base`")),
            );
        }
    }

    if compared.is_empty() {
        return Outcome::Skip(format!("the registry and the repository have nothing of {name} in common to compare"));
    }
    Outcome::from_findings(findings, || format!("the registry and the repository agree on its {}", list(&compared)))
}

/// An address with its trailing slash off, and its scheme and host in one case.
fn normal(address: &str) -> String {
    address.trim_end_matches('/').to_ascii_lowercase()
}

fn list(words: &[&str]) -> String {
    match words {
        [] => String::new(),
        [one] => one.to_string(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_with(files: &[(&str, &str)], listed: Listed) -> (tempfile::TempDir, Repo) {
        let dir = tempfile::tempdir().unwrap();
        for (path, text) in files {
            let path = dir.path().join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        let mut repo = Repo::on_disk(dir.path());
        repo.listed = listed;
        (dir, repo)
    }

    fn kasl() -> Product {
        Product {
            name: "kasl".into(),
            accent: Some("#A9C23F".into()),
            form: Some("cli".into()),
            docs: Some("https://kasl.lacodda.com/".into()),
            ..Product::default()
        }
    }

    const LYRN_TOML: &str = "[project]\nname = \"kasl\"\nform = \"cli\"\naccent = \"#a9c23f\"\n";
    const SITE: &str = "export default defineConfig({ site: 'https://kasl.lacodda.com' })\n";

    #[test]
    fn a_product_that_agrees_with_the_registry_passes() {
        let (_dir, repo) = repo_with(&[("lyrn.toml", LYRN_TOML), ("docs/astro.config.mjs", SITE)], Listed::Product(kasl()));
        assert_eq!(
            check(&repo),
            Outcome::Pass("the registry and the repository agree on its colour and documentation".into())
        );
    }

    #[test]
    fn each_thing_the_two_disagree_on_is_named() {
        let toml = "[project]\nname = \"kasl\"\nform = \"desktop\"\naccent = \"#123456\"\n";
        let site = "export default defineConfig({ site: 'https://lacodda.github.io', base: '/kasl' })\n";
        let (_dir, repo) = repo_with(&[("lyrn.toml", toml), ("docs/astro.config.mjs", site)], Listed::Product(kasl()));
        let outcome = check(&repo);
        let said: Vec<&str> = outcome.findings().iter().map(|f| f.what.as_str()).collect();
        assert_eq!(
            said,
            [
                "the registry gives kasl #A9C23F, and lyrn.toml's accent is #123456",
                "the registry sends readers of kasl to https://kasl.lacodda.com/, and the site is built for https://lacodda.github.io/kasl/",
            ]
        );
    }

    /// Only what both sides say is compared: a product recorded before its
    /// mark was drawn is not wrong for it, and a registry not read is not a
    /// finding.
    #[test]
    fn what_one_side_does_not_say_is_not_compared() {
        let bare = Product {
            name: "kasl".into(),
            ..Product::default()
        };
        let (_dir, repo) = repo_with(&[("lyrn.toml", LYRN_TOML)], Listed::Product(bare));
        assert!(matches!(check(&repo), Outcome::Skip(_)));
        let (_dir, repo) = repo_with(&[("lyrn.toml", LYRN_TOML)], Listed::Unread("rigger is not installed here".into()));
        assert_eq!(
            check(&repo),
            Outcome::Skip("the line's registry was not read - rigger is not installed here".into())
        );
        let (_dir, repo) = repo_with(&[("lyrn.toml", LYRN_TOML)], Listed::Absent("kasl".into()));
        assert_eq!(check(&repo), Outcome::Skip("the line's registry has no kasl".into()));
        // The form is how the repository is built, not what the product is:
        // a service with a web UI is recorded as web, and that is no finding.
        let toml = "[project]\nname = \"kasl\"\nform = \"service\"\n";
        let (_dir, repo) = repo_with(&[("lyrn.toml", toml)], Listed::Product(kasl()));
        assert!(matches!(check(&repo), Outcome::Skip(_)), "{:?}", check(&repo));
    }
}
