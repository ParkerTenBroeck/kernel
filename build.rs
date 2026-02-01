use std::path::Path;

fn image_to_1bpp(path: &Path) -> Vec<bool> {
    let img =
        image::open(path).unwrap_or_else(|e| panic!("Failed to open {}: {e}", path.display()));

    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();

    let mut bits: Vec<bool> = Vec::with_capacity(w as usize * h as usize);

    for y in 0..h {
        for x in 0..w {
            let p = rgba.get_pixel(x, y).0;
            let r = p[0] as u16;
            let g = p[1] as u16;
            let b = p[2] as u16;
            let a = p[3] as u16;

            let luma: u8 = if a == 0 {
                0
            } else {
                ((77 * r + 150 * g + 29 * b) >> 8) as u8
            };

            bits.push(luma >= 128);
        }
    }

    bits
}

fn reorder(
    data: Vec<bool>,
    glyph_width: usize,
    glyph_height: usize,
    glyph_per_row: usize,
) -> Vec<bool> {
    let mut reordered = Vec::new();

    for gy in 0..data.len() / (glyph_height * glyph_width * glyph_per_row) {
        for gx in 0..glyph_per_row {
            for py in 0..glyph_height {
                for px in 0..glyph_width {
                    let index = gx * glyph_width
                        + gy * glyph_height * glyph_per_row * glyph_width
                        + px
                        + py * glyph_per_row * glyph_width;
                    reordered.push(data[index]);
                }
            }
        }
    }

    reordered
}

fn pack(data: Vec<bool>, glyph_width: usize, glyph_height: usize) -> Vec<u8> {
    let mut packed = Vec::new();

    let glyph_size_bits = glyph_width * glyph_height;

    for glyph in data.chunks(glyph_size_bits) {
        for byte in glyph.chunks(8) {
            let mut byte_packed = 0;
            for &bit in byte {
                byte_packed >>= 1;
                byte_packed |= (bit as u8) << 7;
            }
            packed.push(byte_packed >> (8 - byte.len()));
        }
    }

    packed
}

fn fontify(path: impl AsRef<Path>) {
    let path = path.as_ref();

    println!("cargo:rerun-if-changed={}", path.display());

    let glyph_width = 8;
    let glyph_height = 13;
    let glyph_per_row = 16;

    let img = image_to_1bpp(path);
    let reordered = reorder(img, glyph_width, glyph_height, glyph_per_row);
    let packed = pack(reordered, glyph_width, glyph_height);

    let mut out = path.to_path_buf();
    out.set_extension("bin");

    println!("cargo:rerun-if-changed={}", out.display());
    std::fs::write(out, packed).unwrap()
}

pub fn main() {
    fontify("./res/8x13.png");
    fontify("./res/8x13_bold.png");
    fontify("./res/8x13_italic.png");
}
