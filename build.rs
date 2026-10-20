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
