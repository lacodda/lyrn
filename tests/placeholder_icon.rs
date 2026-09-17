//! The placeholder mark the `desktop` form ships, held to the line's rules.
//!
//! The icon is bytes in the repository, produced by
//! `tools/render-placeholder-icon.py`. Bytes do not check themselves, and the
//! line has already shipped this exact class of defect twice: kilna put a flat
//! coloured lozenge on the desktop because the whole `.ico` was built from the
//! S tile, and its titlebar was stretched out of a 16px image because the
//! container was ordered smallest first. Both were found by the owner looking
//! at a screen, months later.
//!
//! So the rules are held here, against the actual pixels, rather than trusted
//! to the script that wrote them. The script could be rewritten, or the file
//! replaced by hand; what matters is what the file says.

use std::path::PathBuf;

/// The container the `desktop` form writes into a generated project.
fn placeholder_ico() -> Vec<u8> {
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "src", "templates", "desktop", "icons", "icon.ico"]
        .iter()
        .collect();
    std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn placeholder_png() -> Vec<u8> {
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "src", "templates", "desktop", "icons", "icon.png"]
        .iter()
        .collect();
    std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// One image inside an `.ico`, as its directory describes it.
#[derive(Debug, PartialEq)]
struct Entry {
    size: u32,
    offset: usize,
    length: usize,
}

/// Read the directory of an `.ico`.
///
/// Sixteen bytes per entry after a six-byte header. Spelled out rather than
/// pulled from a crate: an `.ico` reader is the only thing a crate would
/// bring, the arithmetic is eight lines, and a test that parses the container
/// itself cannot be fooled by a reader that quietly reorders what it returns -
/// which is exactly what the writer on the other side does.
fn entries(ico: &[u8]) -> Vec<Entry> {
    let Some(count) = ico.get(4..6).map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]) as usize) else {
        return Vec::new();
    };

    let mut out = Vec::with_capacity(count);
    for index in 0..count {
        let at = 6 + index * 16;
        let Some(entry) = ico.get(at..at + 16) else {
            break;
        };
        // A zero width means 256: the field is one byte and 256 does not fit.
        let size = if entry[0] == 0 { 256 } else { u32::from(entry[0]) };
        let length = u32::from_le_bytes([entry[8], entry[9], entry[10], entry[11]]) as usize;
        let offset = u32::from_le_bytes([entry[12], entry[13], entry[14], entry[15]]) as usize;
        if ico.len() < offset + length {
            continue;
        }
        out.push(Entry { size, offset, length });
    }
    out
}

/// The line's level boundary: at 27px and below, the outline collapses into
/// noise and the filled tile is all that survives.
const S_CEILING: u32 = 27;

/// Is this image the filled tile (S) rather than the plated mark (M and L)?
///
/// Sampled a quarter of the way across and vertically centred: inside the
/// hexagon and clear of the glyph. The plate is near-black and the filled tile
/// is graphite, so one pixel tells them apart - which is what the eye does at
/// a glance, and what nobody was doing.
fn is_filled(image: &[u8], expected_size: u32) -> bool {
    let decoded = image::load_from_memory_with_format(image, image::ImageFormat::Png).expect("an embedded image is not a PNG");
    let rgba = decoded.to_rgba8();
    let (width, height) = rgba.dimensions();
    assert_eq!(width, expected_size, "a {expected_size}px entry holds a {width}px image");

    let sample = rgba.get_pixel(width / 4, height / 2).0;
    assert!(sample[3] > 40, "the {expected_size}px image is transparent where the tile should be");
    u32::from(sample[0]) + u32::from(sample[1]) + u32::from(sample[2]) > 180
}

/// The container is readable, and every image it promises is really there.
#[test]
fn the_placeholder_container_holds_images() {
    let ico = placeholder_ico();
    let entries = entries(&ico);
    assert!(!entries.is_empty(), "the placeholder .ico gave no images");
    for entry in &entries {
        assert!(entry.length > 0, "the {}px image is empty", entry.size);
    }
}

/// The level rule of the line, per image inside the container.
///
/// This is the test the patch exists for. Written to fail against what the
/// form shipped before it: Tauri's own placeholder is one flat drawing at
/// every size, so whichever way this assertion pointed, some size broke it.
#[test]
fn every_size_carries_the_level_that_reads_at_it() {
    let ico = placeholder_ico();
    for entry in entries(&ico) {
        let filled = is_filled(&ico[entry.offset..entry.offset + entry.length], entry.size);
        if entry.size <= S_CEILING {
            assert!(
                filled,
                "the {}px image is not the filled tile; below 28px the outline collapses into noise",
                entry.size
            );
        } else {
            assert!(
                !filled,
                "the {}px image is the filled S tile, not the plated mark - the line's level rule puts S at 27px and below",
                entry.size
            );
        }
    }
}

/// Largest first.
///
/// Windows picks by nearest size and ignores order, but `tauri-codegen` takes
/// `entries()[0]` verbatim as the window icon, and a titlebar stretched out of
/// a 16px image is how kilna v0.32.1 shipped. Cheap to hold, expensive to
/// notice - and Pillow's own ICO writer sorts ascending no matter what order
/// it is handed, which is how the first draft of the exporter got this wrong.
#[test]
fn the_largest_image_comes_first() {
    let ico = placeholder_ico();
    let sizes: Vec<u32> = entries(&ico).iter().map(|entry| entry.size).collect();
    let mut largest_first = sizes.clone();
    largest_first.sort_unstable_by(|a, b| b.cmp(a));
    assert_eq!(sizes, largest_first, "the images are not ordered largest first");
}

/// The sizes Windows actually asks for are all present.
///
/// 16 and 24 for the titlebar and Alt+Tab, 32 and 48 for the desktop and
/// Explorer, 256 for the task switcher. A missing size is not a hard failure
/// in Windows - it scales the nearest one - but scaling 256 down to 16 is the
/// mush this whole arrangement exists to avoid.
#[test]
fn the_sizes_windows_asks_for_are_there() {
    let ico = placeholder_ico();
    let sizes: Vec<u32> = entries(&ico).iter().map(|entry| entry.size).collect();
    for wanted in [16, 24, 32, 48, 256] {
        assert!(sizes.contains(&wanted), "the .ico has no {wanted}px image: {sizes:?}");
    }
}

/// The PNG Tauri bundles on every non-Windows target is the L drawing.
///
/// 512px is far above the level boundary, so the plated mark is what belongs
/// there; a filled tile at that size is the kilna defect in its other form.
#[test]
fn the_bundled_png_is_the_plated_mark() {
    let png = placeholder_png();
    let decoded = image::load_from_memory_with_format(&png, image::ImageFormat::Png).expect("icon.png is not a PNG");
    let rgba = decoded.to_rgba8();
    assert_eq!(rgba.dimensions(), (512, 512), "icon.png is not 512px");
    assert!(!is_filled(&png, 512), "icon.png is the filled S tile at 512px, not the plated mark");
}

/// The mark is the line's, not something else that happens to be graphite.
///
/// The plate and the umbrella mark's colour are both fixed in the line's brand
/// registry. Holding the actual values here is what keeps a redraw honest: an
/// icon that passes every rule above while being drawn in some other palette
/// is no longer the line's placeholder, and the point of the placeholder is
/// that it is recognisably the line's.
#[test]
fn the_placeholder_is_drawn_in_the_lines_colours() {
    let png = placeholder_png();
    let decoded = image::load_from_memory_with_format(&png, image::ImageFormat::Png).expect("icon.png is not a PNG");
    let rgba = decoded.to_rgba8();

    // Inside the hexagon, clear of the outline and of the glyph: the plate.
    let plate = rgba.get_pixel(128, 256).0;
    assert_eq!([plate[0], plate[1], plate[2]], [0x1B, 0x21, 0x26], "the plate is not the line's #1B2126");

    // The outline, three quarters of the mark's colour over that plate. The
    // exact row is where the centre line crosses it; see the exporter.
    let outline = rgba.get_pixel(56, 256).0;
    let expected: Vec<u8> = [0x8Eu32, 0x9A, 0xA3]
        .iter()
        .zip([0x1Bu32, 0x21, 0x26])
        .map(|(mark, plate)| ((*mark as f32 * 0.75) + (plate as f32 * 0.25)).round() as u8)
        .collect();
    for (channel, want) in expected.iter().enumerate() {
        let got = outline[channel];
        assert!(
            got.abs_diff(*want) <= 2,
            "the outline is not the line's graphite at 75%: channel {channel} is {got}, expected about {want}"
        );
    }
}
