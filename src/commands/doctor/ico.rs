//! An `.ico` read as what it is: a directory of images, each a whole PNG or a
//! bitmap, and the one pixel that tells the line's filled tile from its
//! plated mark.
//!
//! Spelled out rather than taken from a crate: the container is sixteen bytes
//! per image, and a reader that quietly sorted what it returned would hide
//! exactly the order this module exists to check.

/// One image of the container, as its directory describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub size: u32,
    /// The sixteen bytes of its directory record.
    record: [u8; 16],
    offset: usize,
    length: usize,
}

/// The images of an `.ico`, in the order its directory lists them; `None`
/// when the bytes are not one.
pub fn entries(ico: &[u8]) -> Option<Vec<Entry>> {
    if ico.get(0..4)? != [0, 0, 1, 0] {
        return None;
    }
    let count = u16::from_le_bytes([ico[4], ico[5]]) as usize;
    let mut out = Vec::with_capacity(count);
    for index in 0..count {
        let at = 6 + index * 16;
        let record: [u8; 16] = ico.get(at..at + 16)?.try_into().ok()?;
        // A zero width means 256: the field is one byte and 256 does not fit.
        let size = if record[0] == 0 { 256 } else { u32::from(record[0]) };
        let length = u32::from_le_bytes([record[8], record[9], record[10], record[11]]) as usize;
        let offset = u32::from_le_bytes([record[12], record[13], record[14], record[15]]) as usize;
        if length == 0 || ico.len() < offset.checked_add(length)? {
            return None;
        }
        out.push(Entry { size, record, offset, length });
    }
    (!out.is_empty()).then_some(out)
}

impl Entry {
    pub fn bytes<'a>(&self, ico: &'a [u8]) -> &'a [u8] {
        &ico[self.offset..self.offset + self.length]
    }
}

/// The same images, the largest first: what a directory read front to back
/// should meet. Tauri takes the first image as the window's icon.
pub fn largest_first(ico: &[u8], entries: &[Entry]) -> Vec<u8> {
    let mut sorted: Vec<&Entry> = entries.iter().collect();
    sorted.sort_by_key(|e| std::cmp::Reverse(e.size));
    let mut out = Vec::with_capacity(ico.len());
    out.extend_from_slice(&[0, 0, 1, 0]);
    out.extend_from_slice(&(sorted.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * sorted.len();
    for entry in &sorted {
        let mut record = entry.record;
        record[12..16].copy_from_slice(&(offset as u32).to_le_bytes());
        out.extend_from_slice(&record);
        offset += entry.length;
    }
    for entry in &sorted {
        out.extend_from_slice(entry.bytes(ico));
    }
    out
}

/// Which level of the mark an image shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Probe {
    /// The filled tile: the S level.
    Filled,
    /// The dark plate the M and L levels stand on.
    Plated,
    /// Neither clearly, or an image this cannot read: not the line's mark,
    /// or not one to judge.
    Unknown,
}

/// The line's plate, `#1B2126`, the same under every product's M and L.
const PLATE: [u8; 3] = [0x1B, 0x21, 0x26];

/// How far a channel may stray from the plate and still be it: a resampled
/// edge drifts a few steps, a filled tile is dozens away.
const PLATE_TOLERANCE: u8 = 12;

/// The share of an image's opaque pixels that are the plate, at and above
/// which it is the plated mark, and below which it is the filled tile.
/// Measured across the line's seventeen products in October 2026: M and L
/// from 0.64 to 0.80 at every size from 32 to 256, S from 0.00 to 0.04 at 16
/// and 24. Between the two an image is not judged.
const PLATED_FROM: f32 = 0.35;
const FILLED_BELOW: f32 = 0.15;

/// The level an image shows, by how much of it is the plate. M and L stand on
/// it - most of the hexagon is plate around the code and the metaphor - and
/// the S tile is filled with the product's colour, bright for one product and
/// dark for another, but never the plate. One pixel would do on a drawing
/// whose code sits clear of it; a wide code reaches it at 32px, and the share
/// does not care where the code is.
pub fn probe(image: &[u8]) -> Probe {
    let pixels = if image.starts_with(b"\x89PNG") {
        png_pixels(image)
    } else {
        bitmap_pixels(image)
    };
    let Some(pixels) = pixels else { return Probe::Unknown };
    let opaque: Vec<&[u8; 4]> = pixels.iter().filter(|px| px[3] > 128).collect();
    if opaque.is_empty() {
        return Probe::Unknown;
    }
    let plate = opaque
        .iter()
        .filter(|px| px[..3].iter().zip(PLATE).all(|(c, p)| c.abs_diff(p) <= PLATE_TOLERANCE))
        .count();
    let share = plate as f32 / opaque.len() as f32;
    if share >= PLATED_FROM {
        Probe::Plated
    } else if share < FILLED_BELOW {
        Probe::Filled
    } else {
        Probe::Unknown
    }
}

fn png_pixels(image: &[u8]) -> Option<Vec<[u8; 4]>> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(image));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().ok()?;
    let mut buffer = vec![0; reader.output_buffer_size()?];
    let frame = reader.next_frame(&mut buffer).ok()?;
    let channels = frame.color_type.samples();
    let mut pixels = Vec::with_capacity(frame.width as usize * frame.height as usize);
    for row in buffer[..frame.buffer_size()].chunks(frame.line_size) {
        for px in row.chunks_exact(channels).take(frame.width as usize) {
            pixels.push(match frame.color_type {
                png::ColorType::Rgba => [px[0], px[1], px[2], px[3]],
                png::ColorType::Rgb => [px[0], px[1], px[2], 255],
                png::ColorType::GrayscaleAlpha => [px[0], px[0], px[0], px[1]],
                png::ColorType::Grayscale => [px[0], px[0], px[0], 255],
                png::ColorType::Indexed => return None,
            });
        }
    }
    Some(pixels)
}

/// A 32-bit bitmap entry: a BITMAPINFOHEADER, then rows of BGRA. Other
/// depths are older than anything the line writes.
fn bitmap_pixels(image: &[u8]) -> Option<Vec<[u8; 4]>> {
    let header = u32::from_le_bytes(image.get(0..4)?.try_into().ok()?) as usize;
    let width = i32::from_le_bytes(image.get(4..8)?.try_into().ok()?).unsigned_abs() as usize;
    // The height counts the colour rows and the mask below them.
    let height = (i32::from_le_bytes(image.get(8..12)?.try_into().ok()?).unsigned_abs() / 2) as usize;
    let bits = u16::from_le_bytes(image.get(14..16)?.try_into().ok()?);
    if bits != 32 || width == 0 || height == 0 {
        return None;
    }
    let colours = image.get(header..header + width * height * 4)?;
    Some(colours.as_chunks::<4>().0.iter().map(|[b, g, r, a]| [*r, *g, *b, *a]).collect())
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// A square PNG of one colour.
    pub fn png_of(size: u32, rgba: [u8; 4]) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut out, size, size);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            let data: Vec<u8> = (0..size * size).flat_map(|_| rgba).collect();
            writer.write_image_data(&data).unwrap();
        }
        out
    }

    /// An `.ico` of these images, in this order.
    pub fn ico_of(images: &[(u32, Vec<u8>)]) -> Vec<u8> {
        let mut out = vec![0, 0, 1, 0];
        out.extend_from_slice(&(images.len() as u16).to_le_bytes());
        let mut offset = 6 + 16 * images.len();
        for (size, image) in images {
            let side = if *size >= 256 { 0 } else { *size as u8 };
            out.extend_from_slice(&[side, side, 0, 0, 1, 0, 32, 0]);
            out.extend_from_slice(&(image.len() as u32).to_le_bytes());
            out.extend_from_slice(&(offset as u32).to_le_bytes());
            offset += image.len();
        }
        for (_, image) in images {
            out.extend_from_slice(image);
        }
        out
    }

    pub const PLATE_PIXEL: [u8; 4] = [0x1B, 0x21, 0x26, 255];
    pub const TILE_PIXEL: [u8; 4] = [0x6D, 0x7B, 0xF2, 255];

    #[test]
    fn the_container_is_read_in_its_own_order() {
        let ico = ico_of(&[(16, png_of(16, TILE_PIXEL)), (256, png_of(256, PLATE_PIXEL))]);
        let sizes: Vec<u32> = entries(&ico).unwrap().iter().map(|e| e.size).collect();
        assert_eq!(sizes, [16, 256]);
        assert_eq!(entries(b"not an icon"), None);
    }

    #[test]
    fn putting_the_largest_first_keeps_every_image() {
        let small = png_of(16, TILE_PIXEL);
        let large = png_of(48, PLATE_PIXEL);
        let ico = ico_of(&[(16, small.clone()), (48, large.clone())]);
        let sorted = largest_first(&ico, &entries(&ico).unwrap());
        let read = entries(&sorted).unwrap();
        assert_eq!(read.iter().map(|e| e.size).collect::<Vec<_>>(), [48, 16]);
        assert_eq!(read[0].bytes(&sorted), large.as_slice());
        assert_eq!(read[1].bytes(&sorted), small.as_slice());
        assert_eq!(sorted.len(), ico.len());
    }

    /// A 4x4 PNG whose first `plate` pixels are the plate and the rest the tile.
    fn mixed(plate: usize) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut out, 4, 4);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            let data: Vec<u8> = (0..16).flat_map(|i| if i < plate { PLATE_PIXEL } else { TILE_PIXEL }).collect();
            writer.write_image_data(&data).unwrap();
        }
        out
    }

    #[test]
    fn the_share_of_plate_tells_the_tile_from_the_plate() {
        assert_eq!(probe(&png_of(32, TILE_PIXEL)), Probe::Filled);
        assert_eq!(probe(&png_of(32, PLATE_PIXEL)), Probe::Plated);
        // A dark product colour is still not the plate: kasl's S tile at 16px.
        assert_eq!(probe(&png_of(16, [52, 60, 35, 255])), Probe::Filled);
        assert_eq!(probe(&png_of(64, [30, 35, 40, 255])), Probe::Plated);
        // A code across the plate leaves most of it plate; a code in plate
        // colour across the tile leaves little.
        assert_eq!(probe(&mixed(11)), Probe::Plated);
        assert_eq!(probe(&mixed(2)), Probe::Filled);
        assert_eq!(probe(&mixed(4)), Probe::Unknown, "between the two is not judged");
        assert_eq!(probe(&png_of(32, [255, 255, 255, 0])), Probe::Unknown);
        assert_eq!(probe(b"garbage"), Probe::Unknown);
    }

    #[test]
    fn a_bitmap_entry_is_read_as_bgra() {
        let (w, h) = (4usize, 4usize);
        let mut bmp = vec![0u8; 40];
        bmp[0..4].copy_from_slice(&40u32.to_le_bytes());
        bmp[4..8].copy_from_slice(&(w as i32).to_le_bytes());
        bmp[8..12].copy_from_slice(&((h * 2) as i32).to_le_bytes());
        bmp[14..16].copy_from_slice(&32u16.to_le_bytes());
        let plated = |bmp: &mut Vec<u8>, bgra: [u8; 4]| (0..w * h).for_each(|_| bmp.extend_from_slice(&bgra));
        let mut plate = bmp.clone();
        plated(&mut plate, [0x26, 0x21, 0x1B, 255]);
        assert_eq!(probe(&plate), Probe::Plated);
        plated(&mut bmp, [0xF2, 0x7B, 0x6D, 255]);
        assert_eq!(probe(&bmp), Probe::Filled);
    }
}
