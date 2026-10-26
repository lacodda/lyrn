//! The line's mark, drawn: its masters rendered into every raster a
//! repository carries, each size from the level that reads at it.
//!
//! The masters are SVG, and they set their code and their line of text in the
//! line's font. A rasterizer that asks the machine for that font draws a
//! different mark on every machine - the end of a word lands somewhere else
//! when another monospace stands in, and a crop worked out on one machine cuts
//! the banner short on the next. So lyrn carries the font and asks for nothing:
//! the same masters give the same pixels wherever they are drawn.

pub mod ico;

use std::sync::{Arc, OnceLock};

use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::{self, fontdb};
use skrifa::MetadataProvider;

/// The line's plate, `#1B2126`: the tile under every M and L, the banner's
/// ground, and the colour around a social preview.
pub const PLATE: [u8; 3] = [0x1B, 0x21, 0x26];

/// The sizes inside an `.ico`, largest first. Windows picks by the nearest
/// size and ignores the order, but Tauri takes the first image as the
/// window's icon, and a title bar stretched out of 16 pixels is how kilna
/// shipped once.
pub const ICO_SIZES: [u32; 7] = [256, 128, 64, 48, 32, 24, 16];

/// GitHub's social preview: 1280x640, the banner's artwork fitted inside
/// 1040x460 and centred, so the edges GitHub crops in some places it shows
/// the picture are plate.
pub const SOCIAL: (u32, u32) = (1280, 640);
const SOCIAL_ARTWORK: (f32, f32) = (1040.0, 460.0);

/// How far a channel may stray from the plate and still be plate, when the
/// banner is cropped to its artwork. The banner's tile is `#232A30`, eight to
/// ten steps off the plate, and is artwork: it stands just past this.
const PLATE_THRESHOLD: u8 = 7;

/// The font every master sets its code and its line in: Cascadia Code 2407.24
/// as Microsoft released it, under the SIL Open Font License 1.1 (the text is
/// beside it, in `font/LICENSE`). Carried unmodified - its Reserved Font Name
/// is the name the masters ask for, and a cut-down copy could not keep it. The
/// variable font, one file for every weight the masters use: 400 for a
/// banner's line, 700 and 800 for the code.
const FONT: &[u8] = include_bytes!("font/CascadiaCode.ttf");
const FONT_FAMILY: &str = "Cascadia Code";

/// A level of the mark. Which one a size calls for is the line's rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// The plated mark with its metaphor, from 64px up.
    L,
    /// The plated mark with its code alone, from 28 to 63.
    M,
    /// The tile filled with the product's colour, at 27 and below, where
    /// nothing finer survives.
    S,
}

impl Level {
    pub fn for_size(size: u32) -> Level {
        match size {
            0..=27 => Level::S,
            28..=63 => Level::M,
            _ => Level::L,
        }
    }
}

/// The font database every drawing shares: the line's font and nothing else,
/// standing in for every family a master may name, generic ones included.
fn fonts() -> Arc<fontdb::Database> {
    static FONTS: OnceLock<Arc<fontdb::Database>> = OnceLock::new();
    FONTS
        .get_or_init(|| {
            let mut db = fontdb::Database::new();
            db.load_font_source(fontdb::Source::Binary(Arc::new(FONT)));
            db.set_serif_family(FONT_FAMILY);
            db.set_sans_serif_family(FONT_FAMILY);
            db.set_monospace_family(FONT_FAMILY);
            db.set_cursive_family(FONT_FAMILY);
            db.set_fantasy_family(FONT_FAMILY);
            Arc::new(db)
        })
        .clone()
}

/// One master, read and ready to draw.
pub struct Drawing {
    tree: usvg::Tree,
    source: Vec<u8>,
}

impl Drawing {
    /// Read a master. `name` says which one in an error.
    ///
    /// A character the line's font has no glyph for would be drawn as an
    /// empty box, and a text that draws nothing would leave a mark without
    /// its code - both look like a finished picture. Either is refused here
    /// instead, naming the text.
    pub fn parse(svg: &[u8], name: &str) -> Result<Drawing, String> {
        let options = usvg::Options {
            font_family: FONT_FAMILY.to_string(),
            fontdb: fonts(),
            ..usvg::Options::default()
        };
        let tree = usvg::Tree::from_data(svg, &options).map_err(|e| format!("{name} is not an SVG lyrn can read: {e}"))?;
        let font = skrifa::FontRef::new(FONT).expect("the line's font parses");
        let charmap = font.charmap();
        let mut texts = Vec::new();
        collect_texts(tree.root(), &mut texts);
        for text in texts {
            let written: String = text.chunks().iter().map(|c| c.text()).collect();
            if let Some(missing) = written.chars().find(|c| !c.is_whitespace() && charmap.map(*c).is_none()) {
                return Err(format!(
                    "{name}: the text `{}` has `{missing}`, which the line's font (Cascadia Code) has no glyph for",
                    written.trim()
                ));
            }
            if !written.trim().is_empty() && !text.flattened().has_children() {
                return Err(format!("{name}: the text `{}` draws nothing", written.trim()));
            }
        }
        Ok(Drawing { tree, source: svg.to_vec() })
    }

    /// The master as it was read: what a copy of it is.
    pub fn source(&self) -> &[u8] {
        &self.source
    }

    /// The drawing's own size, in its units.
    fn size(&self) -> (f32, f32) {
        let size = self.tree.size();
        (size.width(), size.height())
    }

    /// The drawing as a PNG `size` pixels square, scaled to fill it.
    pub fn png(&self, size: u32) -> Vec<u8> {
        let (width, height) = self.size();
        let mut pixmap = Pixmap::new(size, size).expect("a size above zero");
        let scale = Transform::from_scale(size as f32 / width, size as f32 / height);
        resvg::render(&self.tree, scale, &mut pixmap.as_mut());
        encode(&pixmap)
    }

    /// The drawing on the plate at `scale`, every pixel opaque.
    fn on_plate(&self, width: u32, height: u32, transform: Transform) -> Pixmap {
        let mut pixmap = Pixmap::new(width, height).expect("a size above zero");
        pixmap.fill(resvg::tiny_skia::Color::from_rgba8(PLATE[0], PLATE[1], PLATE[2], 255));
        resvg::render(&self.tree, transform, &mut pixmap.as_mut());
        pixmap
    }
}

fn collect_texts<'a>(group: &'a usvg::Group, out: &mut Vec<&'a usvg::Text>) {
    for node in group.children() {
        match node {
            usvg::Node::Group(group) => collect_texts(group, out),
            usvg::Node::Text(text) => out.push(text),
            usvg::Node::Path(_) | usvg::Node::Image(_) => {}
        }
    }
}

/// A pixmap as a PNG. tiny-skia keeps colours premultiplied; the encoder
/// takes them apart again.
fn encode(pixmap: &Pixmap) -> Vec<u8> {
    pixmap.encode_png().expect("a pixmap encodes")
}

/// A product's mark: its three levels, and its banner when it has one. The
/// line's umbrella mark has none.
pub struct Mark {
    pub l: Drawing,
    pub m: Drawing,
    pub s: Drawing,
    pub banner: Option<Drawing>,
}

/// The masters as files hold them: the bytes of each level, and of the banner.
pub struct Masters<'a> {
    pub l: &'a [u8],
    pub m: &'a [u8],
    pub s: &'a [u8],
    pub banner: Option<&'a [u8]>,
}

impl Mark {
    pub fn read(masters: &Masters, names: [&str; 4]) -> Result<Mark, String> {
        Ok(Mark {
            l: Drawing::parse(masters.l, names[0])?,
            m: Drawing::parse(masters.m, names[1])?,
            s: Drawing::parse(masters.s, names[2])?,
            banner: masters.banner.map(|b| Drawing::parse(b, names[3])).transpose()?,
        })
    }

    pub fn level(&self, level: Level) -> &Drawing {
        match level {
            Level::L => &self.l,
            Level::M => &self.m,
            Level::S => &self.s,
        }
    }

    /// The mark `size` pixels square, from the level its size calls for.
    pub fn png(&self, size: u32) -> Vec<u8> {
        self.level(Level::for_size(size)).png(size)
    }

    /// Every size Windows asks an icon for, each from its own level, the
    /// largest first.
    pub fn ico(&self) -> Vec<u8> {
        let images: Vec<(u32, Vec<u8>)> = ICO_SIZES.iter().map(|&size| (size, self.png(size))).collect();
        ico::write(&images)
    }

    /// GitHub's social preview, drawn from the banner; `None` without one.
    ///
    /// The banner is a plate with the artwork on its left, and how far the
    /// artwork reaches depends on the product's line of text. So the banner is
    /// drawn once on the plate to find where its artwork ends, and drawn again
    /// with that box fitted into the preview and centred - a crop at a width
    /// that fits one product cut the end of another's underline. Drawn, not
    /// resampled: the second drawing is at the preview's own scale.
    pub fn social_preview(&self) -> Option<Vec<u8>> {
        let banner = self.banner.as_ref()?;
        let (width, height) = banner.size();
        // Four times over, so the edge of the artwork is found to a quarter
        // of the banner's unit.
        const PROBE: f32 = 4.0;
        let probe_width = (width * PROBE).ceil() as u32;
        let probe_height = (height * PROBE).ceil() as u32;
        let probe = banner.on_plate(probe_width, probe_height, Transform::from_scale(PROBE, PROBE));
        let (x0, y0, x1, y1) = artwork_box(&probe)?;
        let (x0, y0) = (x0 as f32 / PROBE, y0 as f32 / PROBE);
        let (box_width, box_height) = ((x1 + 1) as f32 / PROBE - x0, (y1 + 1) as f32 / PROBE - y0);

        let (canvas_width, canvas_height) = SOCIAL;
        let scale = (SOCIAL_ARTWORK.0 / box_width).min(SOCIAL_ARTWORK.1 / box_height);
        let tx = (canvas_width as f32 - box_width * scale) / 2.0 - x0 * scale;
        let ty = (canvas_height as f32 - box_height * scale) / 2.0 - y0 * scale;
        let preview = banner.on_plate(canvas_width, canvas_height, Transform::from_row(scale, 0.0, 0.0, scale, tx, ty));
        Some(encode(&preview))
    }
}

/// The smallest box around every pixel that is not the plate, as the first
/// and the last column and row; `None` when all of it is plate.
fn artwork_box(pixmap: &Pixmap) -> Option<(u32, u32, u32, u32)> {
    let width = pixmap.width();
    let mut found: Option<(u32, u32, u32, u32)> = None;
    for (index, pixel) in pixmap.pixels().iter().enumerate() {
        let colour = pixel.demultiply();
        let channels = [colour.red(), colour.green(), colour.blue()];
        if channels.iter().zip(PLATE).all(|(c, p)| c.abs_diff(p) <= PLATE_THRESHOLD) {
            continue;
        }
        let (x, y) = (index as u32 % width, index as u32 / width);
        found = Some(match found {
            None => (x, y, x, y),
            Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
        });
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    const L: &str = include_str!("../templates/dowel/marks/lacodda-L.svg");
    const M: &str = include_str!("../templates/dowel/marks/lacodda-M.svg");
    const S: &str = include_str!("../templates/dowel/marks/lacodda-S.svg");

    /// The pixels of a PNG, as RGBA rows.
    pub fn pixels(png: &[u8]) -> (u32, u32, Vec<[u8; 4]>) {
        let mut decoder = png::Decoder::new(std::io::Cursor::new(png));
        decoder.set_transformations(png::Transformations::normalize_to_color8());
        let mut reader = decoder.read_info().unwrap();
        let mut buffer = vec![0; reader.output_buffer_size().unwrap()];
        let frame = reader.next_frame(&mut buffer).unwrap();
        assert_eq!(frame.color_type, png::ColorType::Rgba);
        let pixels = buffer[..frame.buffer_size()].as_chunks::<4>().0.to_vec();
        (frame.width, frame.height, pixels)
    }

    fn umbrella() -> Mark {
        let masters = Masters {
            l: L.as_bytes(),
            m: M.as_bytes(),
            s: S.as_bytes(),
            banner: None,
        };
        Mark::read(&masters, ["L", "M", "S", "banner"]).unwrap()
    }

    #[test]
    fn a_size_calls_for_its_level() {
        assert_eq!(Level::for_size(16), Level::S);
        assert_eq!(Level::for_size(27), Level::S);
        assert_eq!(Level::for_size(28), Level::M);
        assert_eq!(Level::for_size(63), Level::M);
        assert_eq!(Level::for_size(64), Level::L);
        assert_eq!(Level::for_size(512), Level::L);
    }

    /// Each image of the icon shows the level its size calls for, judged the
    /// way `lyrn doctor` judges every icon of the line, and the largest
    /// comes first.
    #[test]
    fn the_icon_carries_each_level_at_its_sizes_largest_first() {
        let icon = umbrella().ico();
        let entries = ico::entries(&icon).unwrap();
        assert_eq!(entries.iter().map(|e| e.size).collect::<Vec<_>>(), ICO_SIZES);
        for entry in &entries {
            let expected = if entry.size <= 27 { ico::Probe::Filled } else { ico::Probe::Plated };
            assert_eq!(ico::probe(entry.bytes(&icon)), expected, "{}px", entry.size);
            let (width, height, _) = pixels(entry.bytes(&icon));
            assert_eq!((width, height), (entry.size, entry.size));
        }
    }

    /// The text is drawn from the font lyrn carries: the umbrella mark's
    /// lambda leaves the graphite of its glyph on the plate.
    #[test]
    fn the_code_is_drawn_in_the_line_s_font() {
        let (size, _, pixels) = pixels(&umbrella().l.png(256));
        let graphite = pixels
            .iter()
            .filter(|p| p[0].abs_diff(0x8E) <= 6 && p[1].abs_diff(0x9A) <= 6 && p[2].abs_diff(0xA3) <= 6)
            .count();
        // The outline alone is graphite at 75% over the plate, darker than
        // this: only the glyph is graphite itself.
        assert!(graphite > (size * size / 100) as usize, "{graphite} graphite pixels: the lambda was not drawn");
    }

    #[test]
    fn a_character_the_font_does_not_have_is_refused() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><text x="10" y="50" font-family="Cascadia Code">a&#x1F600;</text></svg>"##;
        let error = Drawing::parse(svg.as_bytes(), "logo.svg").err().unwrap();
        assert!(error.contains("no glyph"), "{error}");
    }

    #[test]
    fn any_family_a_master_names_is_drawn_in_the_line_s_font() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><text x="10" y="50" font-family="Helvetica, sans-serif" font-size="40" fill="#fff">ab</text></svg>"##;
        let drawing = Drawing::parse(svg.as_bytes(), "logo.svg").unwrap();
        let (_, _, pixels) = pixels(&drawing.png(100));
        assert!(pixels.iter().any(|p| p[3] > 200), "nothing was drawn");
    }

    /// The preview is the banner's artwork centred on the plate: the margins
    /// left and right agree, and so do top and bottom.
    #[test]
    fn the_social_preview_is_the_banner_centred_on_its_plate() {
        let banner = include_str!("../../assets/banner.svg");
        let masters = Masters {
            l: L.as_bytes(),
            m: M.as_bytes(),
            s: S.as_bytes(),
            banner: Some(banner.as_bytes()),
        };
        let mark = Mark::read(&masters, ["L", "M", "S", "banner"]).unwrap();
        let preview = mark.social_preview().unwrap();
        let (width, height, pixels) = pixels(&preview);
        assert_eq!((width, height), SOCIAL);
        assert!(pixels.iter().all(|p| p[3] == 255), "the preview has see-through pixels");
        let not_plate = |p: &[u8; 4]| p[..3].iter().zip(PLATE).any(|(c, q)| c.abs_diff(q) > PLATE_THRESHOLD);
        let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
        for (i, p) in pixels.iter().enumerate() {
            if not_plate(p) {
                let (x, y) = (i as u32 % width, i as u32 / width);
                (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
            }
        }
        let (left, right, top, bottom) = (x0, width - 1 - x1, y0, height - 1 - y1);
        assert!(left.abs_diff(right) <= 2 && top.abs_diff(bottom) <= 2, "margins {left}/{right}, {top}/{bottom}");
        assert!(left >= 118 && top >= 88, "the artwork reaches the edge GitHub crops: {left}, {top}");
        assert!(left <= 122 || top <= 92, "the artwork does not fill its box: {left}, {top}");
    }

    #[test]
    fn a_mark_without_a_banner_has_no_preview() {
        assert!(umbrella().social_preview().is_none());
    }

    /// Two PNGs show the same picture: the same size, and no channel more than
    /// a couple of steps apart - what another processor's arithmetic may move.
    fn same_picture(a: &[u8], b: &[u8]) -> Result<(), String> {
        let (aw, ah, ap) = pixels(a);
        let (bw, bh, bp) = pixels(b);
        if (aw, ah) != (bw, bh) {
            return Err(format!("{aw}x{ah} against {bw}x{bh}"));
        }
        let worst = ap
            .iter()
            .zip(&bp)
            .flat_map(|(x, y)| x.iter().zip(y).map(|(c, d)| c.abs_diff(*d)))
            .max()
            .unwrap_or(0);
        if worst > 2 { Err(format!("a channel {worst} steps apart")) } else { Ok(()) }
    }

    /// The rasters the templates carry until a project has a mark of its own
    /// are the umbrella mark as `lyrn assets` draws it - not a second drawing
    /// of it by another hand. After the masters or the drawing change:
    /// `LYRN_BLESS=1 cargo test placeholder`.
    #[test]
    fn the_placeholder_rasters_are_the_umbrella_mark_drawn() {
        let mark = umbrella();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("templates");
        let bless = std::env::var_os("LYRN_BLESS").is_some();
        let pngs = [
            ("desktop/icons/icon.png", 512),
            ("spa/public/icon-192.png", 192),
            ("spa/public/apple-touch-icon.png", 180),
        ];
        for (path, size) in pngs {
            let drawn = mark.png(size);
            if bless {
                std::fs::write(root.join(path), &drawn).unwrap();
                continue;
            }
            same_picture(&std::fs::read(root.join(path)).unwrap(), &drawn)
                .unwrap_or_else(|e| panic!("{path} is not the umbrella mark drawn at {size}px ({e}) - `LYRN_BLESS=1 cargo test placeholder`"));
        }
        let path = "desktop/icons/icon.ico";
        let drawn = mark.ico();
        if bless {
            std::fs::write(root.join(path), &drawn).unwrap();
            return;
        }
        let kept = std::fs::read(root.join(path)).unwrap();
        let (kept_entries, drawn_entries) = (ico::entries(&kept).unwrap(), ico::entries(&drawn).unwrap());
        assert_eq!(
            kept_entries.iter().map(|e| e.size).collect::<Vec<_>>(),
            drawn_entries.iter().map(|e| e.size).collect::<Vec<_>>(),
            "{path} - `LYRN_BLESS=1 cargo test placeholder`"
        );
        for (k, d) in kept_entries.iter().zip(&drawn_entries) {
            same_picture(k.bytes(&kept), d.bytes(&drawn)).unwrap_or_else(|e| panic!("{path} at {}px: {e} - `LYRN_BLESS=1 cargo test placeholder`", k.size));
        }
    }
}
