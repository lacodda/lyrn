//! `site`: the documentation site agrees with itself and with the manifests
//! about where it lives.
//!
//! Two arrangements are consistent: a project site on
//! `owner.github.io/name`, built with `base: '/name'`, or a domain of its own
//! named in `public/CNAME`, built for that domain with no `base` and with
//! links written from the root. The failure is a mixture of the two, and it is
//! silent: lyrn's own site moved to its domain, kept its `base`, and for a
//! month every page asked for stylesheets under a path the domain does not
//! serve. The pages built and the links resolved; the owner saw unstyled HTML
//! in a browser. And the homepage a registry shows has to lead to wherever
//! the site is.

use regex::Regex;

use super::product::{Kind, Product};
use crate::commands::doctor::{Finding, Outcome, Repo, is_test_material};

/// A Starlight site, as its config says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Site {
    /// The directory it lives in: `docs` or `docs/site`.
    pub dir: String,
    pub site: Option<String>,
    pub base: Option<String>,
    pub description: Option<String>,
}

impl Site {
    /// The site of the repository, if it has one.
    pub fn read(repo: &Repo) -> Option<Site> {
        let config = repo
            .files_named("astro.config.mjs")
            .filter(|f| !is_test_material(f) && !f.split('/').any(|p| p == "examples"))
            .min_by_key(|f| f.matches('/').count())?;
        let text = repo.read_text(config)?;
        let dir = config.rsplit_once('/').map_or(".", |(d, _)| d).to_string();
        Some(parse(dir, &text))
    }

    /// Where the documentation pages are kept, for the commands' pages.
    pub fn reference_dir(&self) -> String {
        format!("{}/src/content/docs/reference", self.dir)
    }
}

fn parse(dir: String, text: &str) -> Site {
    let code = without_comments(text);
    Site {
        dir,
        site: value(&code, "site"),
        base: value(&code, "base"),
        description: value(&code, "description"),
    }
}

/// The source with its comments out: a `base` commented out is no base.
fn without_comments(text: &str) -> String {
    let block = Regex::new(r"(?s)/\*.*?\*/").expect("a valid pattern");
    let text = block.replace_all(text, "");
    text.lines()
        .map(|line| match line.find("//") {
            // `https://` is not a comment.
            Some(at) if !line[..at].ends_with(':') => &line[..at],
            _ => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// What `key` is set to: a string written in place (`site: '…'`), a constant
/// named there (`description`), or the part of a `new URL('…')` the key
/// takes (`site: home.origin`, `base: home.pathname`).
fn value(code: &str, key: &str) -> Option<String> {
    let literal = r#"(?s)\s*(?:'((?:[^'\\]|\\.)*)'|"((?:[^"\\]|\\.)*)"|`([^`]*)`)"#;
    let pick = |c: &regex::Captures| {
        c.get(1)
            .or_else(|| c.get(2))
            .or_else(|| c.get(3))
            .map(|m| m.as_str().replace("\\'", "'").replace("\\\"", "\""))
    };
    let in_place = Regex::new(&format!(r"(?:^|[\s,{{]){key}\s*:{literal}")).expect("a valid pattern");
    if let Some(found) = in_place.captures(code).and_then(|c| pick(&c)) {
        return Some(found);
    }
    // `description,` in an object, or `key: name`, naming a constant.
    let named = Regex::new(&format!(r"(?:^|[\s,{{]){key}\s*:\s*([A-Za-z_$][\w$]*)(?:\.(origin|pathname))?")).expect("a valid pattern");
    let (constant, part) = match named.captures(code) {
        Some(c) => (c[1].to_string(), c.get(2).map(|m| m.as_str().to_string())),
        None if Regex::new(&format!(r"[{{,]\s*{key}\s*[,}}]")).expect("a valid pattern").is_match(code) => (key.to_string(), None),
        None => return None,
    };
    let declared = Regex::new(&format!(r"(?:const|let|var)\s+{}\s*={literal}", regex::escape(&constant))).expect("a valid pattern");
    if let Some(found) = declared.captures(code).and_then(|c| pick(&c)) {
        return Some(found);
    }
    let url = Regex::new(&format!(r"(?:const|let|var)\s+{}\s*=\s*new\s+URL\({literal}", regex::escape(&constant))).expect("a valid pattern");
    let address = url.captures(code).and_then(|c| pick(&c))?;
    let (origin, path) = split_url(&address)?;
    match part.as_deref() {
        Some("origin") => Some(origin),
        Some("pathname") => Some(path),
        _ => Some(address),
    }
}

/// `https://host/a/b` as its origin and its path.
fn split_url(url: &str) -> Option<(String, String)> {
    let (scheme, rest) = url.split_once("://")?;
    let (host, path) = rest.split_once('/').map_or((rest, ""), |(h, p)| (h, p));
    Some((format!("{scheme}://{host}"), format!("/{path}")))
}

fn trim_slashes(text: &str) -> &str {
    text.trim_end_matches('/')
}

pub fn check(repo: &Repo, product: &Product) -> Outcome {
    let Some(site) = Site::read(repo) else {
        return Outcome::Skip("no documentation site".to_string());
    };
    let Some(address) = site.site.as_deref() else {
        return Outcome::Found(vec![
            Finding::fail(format!("{}/astro.config.mjs sets no `site` lyrn can read", site.dir))
                .fix("write the address the site is served from: `site: 'https://…'`"),
        ]);
    };
    let base = site.base.as_deref().map(|b| b.trim_matches('/')).filter(|b| !b.is_empty());
    let cname_path = format!("{}/public/CNAME", site.dir);
    let cname = repo.read_text(&cname_path).map(|t| t.trim().to_string()).filter(|t| !t.is_empty());
    let repo_name = product.github.as_deref().and_then(|g| g.split('/').nth(1)).unwrap_or(&product.name).to_string();
    let mut findings = Vec::new();

    let served = match &cname {
        Some(domain) => {
            let expected = format!("https://{domain}");
            if trim_slashes(address) != expected {
                findings.push(
                    Finding::fail(format!("{cname_path} serves {domain}, and astro.config.mjs builds for {address}")).fix(format!("`site: '{expected}'`")),
                );
            }
            if let Some(base) = base {
                findings.push(
                    Finding::fail(format!("astro.config.mjs keeps `base: '/{base}'` while {domain} serves the site from its root"))
                        .fix("remove `base`: every asset would be asked for under a path the domain does not have"),
                );
            }
            findings.extend(prefixed_links(repo, &site, &repo_name));
            format!("{expected}/")
        }
        None => {
            let owner = product.github.as_deref().and_then(|g| g.split('/').next());
            let host_ok = match owner {
                Some(owner) => trim_slashes(address).eq_ignore_ascii_case(&format!("https://{owner}.github.io")),
                None => trim_slashes(address).ends_with(".github.io"),
            };
            if !host_ok {
                let wanted = owner.map_or("https://<owner>.github.io".to_string(), |o| format!("https://{o}.github.io"));
                findings.push(
                    Finding::fail(format!(
                        "no {cname_path}, so the site is a project site, and astro.config.mjs builds for {address}"
                    ))
                    .fix(format!("`site: '{wanted}'`, or name the domain in {cname_path}")),
                );
            }
            if base != Some(repo_name.as_str()) {
                findings.push(
                    Finding::fail(match base {
                        Some(base) => format!("a project site of `{repo_name}` is served under /{repo_name}, and astro.config.mjs builds for /{base}"),
                        None => format!("a project site is served under /{repo_name}, and astro.config.mjs sets no `base`"),
                    })
                    .fix(format!("`base: '/{repo_name}'`")),
                );
            }
            format!("{}/{repo_name}/", trim_slashes(address))
        }
    };

    let mut pointed = 0;
    for manifest in &product.manifests {
        if !manifest.publishable || !manifest.name.as_deref().is_some_and(|n| product.owns(n)) {
            continue;
        }
        let fields: Vec<(&str, &Option<String>)> = match manifest.kind {
            Kind::Cargo => vec![("homepage", &manifest.homepage), ("documentation", &manifest.documentation)],
            _ => vec![("homepage", &manifest.homepage)],
        };
        for (field, value) in fields {
            let Some(value) = value else { continue };
            // docs.rs documents a library's API, which is not the site.
            if value.contains("docs.rs/") {
                continue;
            }
            if trim_slashes(value) == trim_slashes(&served) {
                pointed += 1;
            } else {
                findings.push(
                    Finding::fail(format!("{} sends readers to {value} (`{field}`), and the site is at {served}", manifest.path))
                        .fix(format!("`{field}` = \"{}\"", trim_slashes(&served))),
                );
            }
        }
    }

    Outcome::from_findings(findings, || {
        let arrangement = if cname.is_some() { "its own domain" } else { "a project site" };
        let manifests = match pointed {
            0 => String::new(),
            1 => ", and 1 manifest points there".to_string(),
            n => format!(", and {n} manifests point there"),
        };
        format!("{served}, {arrangement}{manifests}")
    })
}

/// Links written for a project site, left in a site that now has a domain.
fn prefixed_links(repo: &Repo, site: &Site, repo_name: &str) -> Vec<Finding> {
    let prefix = format!("/{repo_name}/");
    let prefix_at = Regex::new(&format!(r#"(?:^|[\s("'=`\]])/{}/"#, regex::escape(repo_name))).expect("a valid pattern");
    let mut findings = Vec::new();
    for file in &repo.files {
        if !file.starts_with(&format!("{}/", site.dir)) || !(file.ends_with(".md") || file.ends_with(".mdx") || file.ends_with(".mjs")) {
            continue;
        }
        let Some(text) = repo.read_text(file) else { continue };
        // Absolute addresses carry the name legitimately:
        // github.com/owner/name/... and the raw files of the installers.
        let relative: String = text.split_whitespace().filter(|w| !w.contains("://")).collect::<Vec<_>>().join(" ");
        if prefix_at.is_match(&relative) {
            findings.push(
                Finding::fail(format!("{file} links to `{prefix}…`, a path the domain does not serve"))
                    .fix("with no `base`, a link inside the site is written from the root"),
            );
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_config_is_read_however_it_is_written() {
        let site = parse(
            "docs".into(),
            "const title = 'lyrn';\nconst description = \"Start a new app.\";\nexport default defineConfig({\n\tsite: 'https://lyrn.lacodda.com',\n\t// base: '/lyrn',\n\tintegrations: [starlight({ title, description })],\n});\n",
        );
        assert_eq!(site.site.as_deref(), Some("https://lyrn.lacodda.com"));
        assert_eq!(site.base, None);
        assert_eq!(site.description.as_deref(), Some("Start a new app."));

        let site = parse(
            "docs".into(),
            "export default defineConfig({\n\tsite: 'https://lacodda.github.io',\n\tbase: '/kasl',\n\tintegrations: [starlight({\n\t\ttitle: 'kasl',\n\t\tdescription:\n\t\t\t'Work activity tracker CLI',\n\t})],\n});\n",
        );
        assert_eq!((site.site.as_deref(), site.base.as_deref()), (Some("https://lacodda.github.io"), Some("/kasl")));
        assert_eq!(site.description.as_deref(), Some("Work activity tracker CLI"));

        let site = parse(
            "docs".into(),
            "const home = new URL('https://lacodda.github.io/efema/');\nexport default defineConfig({\n\tsite: home.origin,\n\tbase: home.pathname,\n});\n",
        );
        assert_eq!(
            (site.site.as_deref(), site.base.as_deref()),
            (Some("https://lacodda.github.io"), Some("/efema/"))
        );
    }
}
