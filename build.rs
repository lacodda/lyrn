// Embeds the Windows executable icon and version metadata, and the archive
// of the line's standard.
fn main() {
    // The icon is the S tile of the lacodda line mark, exported to a
    // multi-size .ico so Explorer picks the right resolution for each view.
    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=assets/icon.ico");
        winresource::WindowsResource::new()
            .set_icon("assets/icon.ico")
            .compile()
            .expect("failed to embed the Windows resources");
    }

    archive();
    marks();
}

/// Every product's masters as a table of `include_str!`s, from the directory
/// `tools/vendor-dowel.mjs` writes them to.
fn marks() {
    use std::fmt::Write as _;
    use std::path::Path;

    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("templates").join("dowel").join("marks");
    println!("cargo:rerun-if-changed=src/templates/dowel/marks");
    let mut products: Vec<String> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .map(|entry| entry.expect("a readable directory entry").file_name().to_string_lossy().into_owned())
        .filter_map(|name| name.strip_suffix("-S.svg").map(str::to_string))
        .collect();
    products.sort();

    let mut out = String::from("/// Each mark's masters: its name, then L, M and S.\nstatic MASTERS: &[(&str, &str, &str, &str)] = &[\n");
    for product in products {
        let path = |level: &str| dir.join(format!("{product}-{level}.svg")).display().to_string();
        let (l, m, s) = (path("L"), path("M"), path("S"));
        writeln!(out, "    ({product:?}, include_str!({l:?}), include_str!({m:?}), include_str!({s:?})),").unwrap();
    }
    out.push_str("];\n");
    let target = Path::new(&std::env::var("OUT_DIR").expect("cargo sets OUT_DIR")).join("marks.rs");
    std::fs::write(target, out).expect("cannot write the table of marks");
}

/// Every version of the standard and every template it keeps, as a table of
/// `include_str!`s - read from the directory rather than listed by hand, so a
/// version added to the archive is in the binary without a second edit.
fn archive() {
    use std::fmt::Write as _;
    use std::path::Path;

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("standard").join("archive");
    println!("cargo:rerun-if-changed=src/standard/archive");

    let entries = |dir: &Path, extension: &str| -> Vec<(String, String)> {
        let mut found: Vec<(String, String)> = std::fs::read_dir(dir)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
            .map(|entry| entry.expect("a readable directory entry").path())
            .filter(|path| path.extension().is_some_and(|e| e == extension))
            .map(|path| {
                let stem = path.file_stem().expect("a file name").to_string_lossy().into_owned();
                (stem, path.display().to_string())
            })
            .collect();
        found.sort();
        found
    };

    let mut out = String::new();
    out.push_str("/// Each version's index: what it changed and which template each file is.\n");
    out.push_str("static INDEXES: &[(&str, &str)] = &[\n");
    for (version, path) in entries(&root, "toml") {
        writeln!(out, "    ({version:?}, include_str!({path:?})),").unwrap();
    }
    out.push_str("];\n\n/// The templates, by the hash of their text.\n");
    out.push_str("static BLOBS: &[(&str, &str)] = &[\n");
    for (hash, path) in entries(&root.join("blobs"), "tmpl") {
        writeln!(out, "    ({hash:?}, include_str!({path:?})),").unwrap();
    }
    out.push_str("];\n");

    let target = Path::new(&std::env::var("OUT_DIR").expect("cargo sets OUT_DIR")).join("standard_archive.rs");
    std::fs::write(target, out).expect("cannot write the archive table");
}
