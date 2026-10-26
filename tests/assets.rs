//! `lyrn assets`, run as a user runs it: a generated project takes a mark of
//! its own in from a directory of masters, and every raster is drawn by the
//! line's rule - which the doctor, reading the files, then agrees with.

use std::fs;
use std::path::{Path, PathBuf};

mod common;

/// lyrn's own masters, standing in for a product's as its generator writes them.
fn masters(dir: &Path, mark: &str) -> PathBuf {
    let out = dir.join("brand");
    fs::create_dir_all(&out).unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    for (from, level) in [("logo.svg", "L"), ("logo-m.svg", "M"), ("logo-s.svg", "S"), ("banner.svg", "banner")] {
        fs::copy(root.join(from), out.join(format!("{mark}-{level}.svg"))).unwrap();
    }
    out
}

/// A cli project, as `lyrn new` writes it: the umbrella mark in its place.
fn project(dir: &Path) -> PathBuf {
    let root = dir.join("demo-tool");
    common::lyrn()
        .args([
            "new",
            "demo-tool",
            "--form",
            "cli",
            "--yes",
            "--no-hooks",
            "--repo",
            "owner/demo-tool",
            "--path",
        ])
        .arg(&root)
        .env("GIT_AUTHOR_NAME", "Ada Lovelace")
        .env("GIT_AUTHOR_EMAIL", "ada@example.com")
        .env("GIT_COMMITTER_NAME", "Ada Lovelace")
        .env("GIT_COMMITTER_EMAIL", "ada@example.com")
        .assert()
        .success();
    root
}

fn assets(root: &Path, args: &[&str]) -> (String, bool) {
    let out = common::lyrn().arg("assets").arg(root).args(args).output().unwrap();
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    (text, out.status.success())
}

/// The images inside an `.ico`, in the order its directory lists them.
fn ico_images(ico: &[u8]) -> Vec<(u32, Vec<u8>)> {
    let count = u16::from_le_bytes([ico[4], ico[5]]) as usize;
    (0..count)
        .map(|i| {
            let at = 6 + 16 * i;
            let size = if ico[at] == 0 { 256 } else { u32::from(ico[at]) };
            let length = u32::from_le_bytes(ico[at + 8..at + 12].try_into().unwrap()) as usize;
            let offset = u32::from_le_bytes(ico[at + 12..at + 16].try_into().unwrap()) as usize;
            (size, ico[offset..offset + length].to_vec())
        })
        .collect()
}

/// The share of a PNG's opaque pixels that are the line's plate.
fn plate_share(png: &[u8]) -> f32 {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(png));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().unwrap();
    let mut buffer = vec![0; reader.output_buffer_size().unwrap()];
    let frame = reader.next_frame(&mut buffer).unwrap();
    let pixels: Vec<&[u8; 4]> = buffer[..frame.buffer_size()].as_chunks::<4>().0.iter().filter(|p| p[3] > 128).collect();
    let plate = pixels
        .iter()
        .filter(|p| p[0].abs_diff(0x1B) <= 12 && p[1].abs_diff(0x21) <= 12 && p[2].abs_diff(0x26) <= 12)
        .count();
    plate as f32 / pixels.len() as f32
}

fn png_size(png: &[u8]) -> (u32, u32) {
    (
        u32::from_be_bytes(png[16..20].try_into().unwrap()),
        u32::from_be_bytes(png[20..24].try_into().unwrap()),
    )
}

#[test]
fn a_mark_taken_in_is_drawn_at_every_size_by_its_level() {
    let dir = tempfile::tempdir().unwrap();
    let root = project(dir.path());
    let brand = masters(dir.path(), "demo-tool");
    assert!(fs::read_to_string(root.join("lyrn.toml")).unwrap().contains("mark = \"placeholder\""));

    let (report, passed) = assets(&root, &["--from", brand.to_str().unwrap()]);
    assert!(passed, "{report}");
    for line in [
        "wrote      assets/logo.svg",
        "demo-tool-L.svg, the master of L",
        "wrote      assets/banner.svg",
        "wrote      assets/icon.ico",
        "wrote      assets/social-preview.png",
        "lyrn.toml: `mark = \"chosen\"`",
        "upload it by hand at https://github.com/owner/demo-tool/settings",
    ] {
        assert!(report.contains(line), "no `{line}` in:\n{report}");
    }

    // The masters are the generator's, byte for byte.
    let own = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    for (ours, theirs) in [
        ("logo.svg", "logo.svg"),
        ("logo-m.svg", "logo-m.svg"),
        ("logo-s.svg", "logo-s.svg"),
        ("banner.svg", "banner.svg"),
    ] {
        assert_eq!(fs::read(root.join("assets").join(ours)).unwrap(), fs::read(own.join(theirs)).unwrap(), "{ours}");
    }
    assert!(fs::read_to_string(root.join("lyrn.toml")).unwrap().contains("mark = \"chosen\""));

    // The icon build.rs embeds: largest first, the plated mark down to 28px,
    // the filled tile below it.
    let images = ico_images(&fs::read(root.join("assets/icon.ico")).unwrap());
    assert_eq!(images.iter().map(|(s, _)| *s).collect::<Vec<_>>(), [256, 128, 64, 48, 32, 24, 16]);
    for (size, png) in &images {
        assert_eq!(png_size(png), (*size, *size));
        let share = plate_share(png);
        if *size <= 27 {
            assert!(share < 0.15, "{size}px: {share} of it is plate, where the filled tile belongs");
        } else {
            assert!(share > 0.35, "{size}px: {share} of it is plate, where the plated mark belongs");
        }
    }
    for (file, size, plated) in [("favicon-32.png", 32, false), ("apple-touch-icon.png", 180, true), ("logo-512.png", 512, true)] {
        let png = fs::read(root.join("assets").join(file)).unwrap();
        assert_eq!(png_size(&png), (size, size), "{file}");
        assert_eq!(plate_share(&png) > 0.35, plated, "{file}");
    }
    assert_eq!(png_size(&fs::read(root.join("assets/social-preview.png")).unwrap()), (1280, 640));

    // The doctor reads the same files and finds them by the rule.
    let doctor = common::lyrn().arg("doctor").arg(&root).env_remove("CI").output().unwrap();
    let doctor = String::from_utf8_lossy(&doctor.stdout).into_owned();
    assert!(doctor.contains("✓ assets"), "{doctor}");
    assert!(doctor.contains("✓ mark       a mark of its own"), "{doctor}");

    // Drawn again, nothing moves: the drawing is the same every time.
    let (again, passed) = assets(&root, &[]);
    assert!(passed, "{again}");
    assert!(!again.contains("wrote"), "a second run wrote something:\n{again}");
    assert!(again.contains("unchanged  assets/icon.ico"), "{again}");
}

#[test]
fn a_directory_without_the_mark_says_which_marks_it_holds() {
    let dir = tempfile::tempdir().unwrap();
    let root = project(dir.path());
    let brand = masters(dir.path(), "other");
    let (report, passed) = assets(&root, &["--from", brand.to_str().unwrap()]);
    assert!(!passed);
    assert!(report.contains("has no demo-tool-L.svg"), "{report}");
    assert!(report.contains("the marks there: other - `--mark` names one"), "{report}");

    let (report, passed) = assets(&root, &["--from", brand.to_str().unwrap(), "--mark", "other"]);
    assert!(passed, "{report}");
    assert!(report.contains("other-L.svg, the master of L"), "{report}");
}

#[test]
fn a_repository_without_masters_is_told_where_they_go() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("README.md"), "# demo\n").unwrap();
    let (report, passed) = assets(dir.path(), &[]);
    assert!(!passed);
    assert!(report.contains("no masters in assets/"), "{report}");
    assert!(report.contains("lyrn assets --from <directory>"), "{report}");
}
