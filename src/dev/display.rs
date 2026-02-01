use core::ops::Sub;

use ::ansi::*;

use crate::{
    println,
    vga::{self, FrameBuffer},
};

#[derive(Debug)]
pub struct VGAAnsiController {
    frame: FrameBuffer,
    font: &'static Font,

    line: u16,
    col: u16,

    bg: vga::Color,
    fg: vga::Color,

    inverted_c: bool,
    bold: bool,
    italic: bool,
    strike: Option<vga::Color>,
    underline: Option<vga::Color>,

    scroll_start: u16,
    scroll_end: u16,
}

#[derive(Debug)]
pub struct Font {
    data: &'static [u8],
    bold: &'static [u8],
    italic: &'static [u8],

    glyph_per_row: usize,

    glyph_width: usize,
    glyph_height: usize,
}

impl VGAAnsiController {
    pub const fn new(frame: FrameBuffer, font: &'static Font) -> Self {
        Self {
            frame,
            font,
            line: 0,
            col: 0,
            bg: BLACK,
            fg: WHITE,
            scroll_start: 0,
            scroll_end: (frame.yres() / font.glyph_height) as u16,

            inverted_c: false,
            strike: None,
            underline: None,
            bold: false,
            italic: false,
        }
    }

    pub fn update_buffer(&mut self, frame: FrameBuffer) {
        self.frame = frame;
        self.scroll_end = self.max_line();
        self.frame.clear(BLACK);
    }

    pub const fn max_col(&self) -> u16 {
        (self.frame.xres() / self.font.glyph_width) as u16
    }

    pub const fn max_line(&self) -> u16 {
        (self.frame.yres() / self.font.glyph_height) as u16
    }

    pub fn advance(&mut self, data: Out) {
        match data {
            Out::Data(char) => self.print_char(char),
            Out::None => {}

            Out::DCSData(_) => {}
            Out::SData(_) => {}
            Out::PMData(_) => {}
            Out::APCData(_) => {}
            Out::OSData(_) => {}

            Out::CSISequenceTooLarge => {}
            Out::CSIIntermediateOverflow => {}
            Out::InvalidEscapeByte(_) => {}
            Out::InvalidUtf8Sequence => {}
            Out::InvalidCodepoint(_) => {}

            Out::CSI(csi) => self.handle_csi(csi),
            Out::nF(items) => self.handle_nf(items),
            Out::nFSequenceTooLarge => todo!(),
            Out::nFInvalidSequence => todo!(),
            Out::C0(c0) => self.handle_c0(c0),
            Out::C1(_) => {}
            Out::Fp(fp) => self.handle_fp(fp),
            Out::Fs(fs) => self.handle_fs(fs),

            Out::SP => self.print_char(' '),
            Out::DEL => todo!(),
        }
    }

    fn print_char(&mut self, char: char) {
        if self.col > self.max_col() {
            self.col = 0;
            self.line += 1;
            if self.line >= self.scroll_end {
                // let bruh = core::slice::from_raw_parts_mut(
                //     FRAME_BUF as *mut u8,
                //     (WIDTH * HEIGHT) as usize,
                // );
                // const TMP: usize = WIDTH as usize * CHAR.character_size.height as usize;
                // bruh.copy_within(
                //     (self.scroll_start as usize + 1) * TMP..((self.scroll_end as usize + 1) * TMP),
                //     (self.scroll_start as usize) * TMP,
                // );

                // bruh[self.scroll_end as usize * TMP..(self.scroll_end as usize + 1) * TMP]
                //     .fill(self.bg.0);
                self.line -= 1;
            }
        }

        let glyph = if self.bold {
            self.font.bold_glyph_data(char)
        } else if self.italic {
            self.font.italic_glyph_data(char)
        } else {
            self.font.glyph_data(char)
        };

        let fg = if self.inverted_c { self.bg } else { self.fg };
        let bg = if !self.inverted_c { self.bg } else { self.fg };

        let xstart = self.col as usize * self.font.glyph_width;
        let ystart = self.line as usize * self.font.glyph_height;

        for y in 0..self.font.glyph_height {
            for x in 0..self.font.glyph_width {
                let index = x + y * self.font.glyph_width;
                let byte_i = index / 8;
                let bit_i = index % 8;
                let color = if (glyph[byte_i] >> bit_i) & 0x1 == 1 {
                    fg
                } else {
                    bg
                };
                self.frame.set(xstart + x, ystart + y, color);
            }
        }

        if let Some(color) = self.strike {
            let y = ystart + self.font.glyph_height / 2;
            for x in 0..self.font.glyph_width {
                self.frame.set(xstart + x, y, color);
            }
        }

        if let Some(color) = self.underline {
            let y = ystart + self.font.glyph_height - 1;
            for x in 0..self.font.glyph_width {
                self.frame.set(xstart + x, y, color);
            }
        }

        self.col += 1;
    }

    fn handle_c0(&mut self, c0: C0) {
        match c0 {
            C0::BS => {
                if self.line == 0 {
                    if self.col > 0 {
                        self.col -= 1;
                    }
                } else if self.col == 0 {
                    self.col = self.max_col() - 1;
                    self.line -= 1;
                } else {
                    self.col -= 1;
                }
            }
            C0::BEL => {}
            C0::CR => {
                self.col = 0;
            }
            C0::FF => {}
            C0::LF => {
                let bg = if !self.inverted_c { self.bg } else { self.fg };

                self.col = 0;
                self.line += 1;
                if self.line >= self.scroll_end {
                    // let bruh = core::slice::from_raw_parts_mut(
                    //     FRAME_BUF as *mut u8,
                    //     (WIDTH * HEIGHT) as usize,
                    // );
                    // const TMP: usize = WIDTH as usize * CHAR.character_size.height as usize;
                    // bruh.copy_within(
                    //     (self.scroll_start as usize + 1) * TMP
                    //         ..((self.scroll_end as usize + 1) * TMP),
                    //     (self.scroll_start as usize) * TMP,
                    // );
                    // bruh[self.scroll_end as usize * TMP..(self.scroll_end as usize + 1) * TMP]
                    //     .fill(self.bg);
                    self.line -= 1;
                }
            }
            C0::HT => self.col = (self.col + 3) & !(4 - 1),
            _ => {}
        }
    }

    fn handle_nf(&mut self, nf: &[u8]) {
        println!("Nf: {nf:?}")
    }

    fn handle_fp(&mut self, fp: Fp) {
        println!("{fp:?}")
    }

    fn handle_fs(&mut self, fs: Fs) {
        println!("{fs:?}")
    }

    fn handle_csi(&mut self, csi: CSI) {
        match csi.parse() {
            KnownCSI::ReportCursorPosition => {
                // CSI r ; c R
                println!("{};{}R", self.line + 1, self.col + 1)
            }
            KnownCSI::CursorTo { row, col } | KnownCSI::HorizontalVerticalPosition { row, col } => {
                self.col = col.min(self.max_col());
                self.line = row.min(self.max_line());
            }
            KnownCSI::InsertLines(lines) => {
                // let bruh = core::slice::from_raw_parts_mut(
                //     FRAME_BUF as *mut u8,
                //     (WIDTH * HEIGHT) as usize,
                // );
                // bruh.copy_within(
                //     (self.scroll_start as usize) * TMP
                //         ..(self.scroll_end as usize + 1 - lines as usize) * TMP,
                //     (lines as usize + self.scroll_start as usize) * TMP,
                // );
                // const TMP: usize = WIDTH as usize * CHAR.character_size.height as usize;
                // bruh[self.scroll_start as usize * TMP
                //     ..(lines as usize + self.scroll_start as usize) * TMP]
                //     .fill(self.bg.0);
            }
            KnownCSI::CursorUp(amount) => self.line = self.line.saturating_sub(amount),
            KnownCSI::CursorDown(amount) => {
                self.line = self.line.saturating_add(amount).max(self.max_line())
            }
            KnownCSI::CursorLeft(amount) => self.col = self.col.saturating_sub(amount),
            KnownCSI::CursorRight(amount) => {
                self.col = self.col.saturating_add(amount).max(self.max_col())
            }
            KnownCSI::CursorNextLine(amount) => {
                self.col = 0;
                self.line = self.line.saturating_add(amount).max(self.max_line())
            }
            KnownCSI::CursorPreviousLine(amount) => {
                self.col = 0;
                self.line = self.line.saturating_sub(amount)
            }
            KnownCSI::CursorHorizontalAbsolute(col) => self.col = (col - 1).max(self.max_col()),
            KnownCSI::CursorLineAbsolute(line) => self.line = (line - 1).max(self.max_line()),
            KnownCSI::EraseDisplay | KnownCSI::EraseScreen => {
                self.col = 0;
                self.line = 0;
                self.frame.clear(BLACK);
            }
            KnownCSI::EraseFromCursor | KnownCSI::EraseFromCursorToEndOfLine => {
                let yoff = self.line as usize * self.font.glyph_height;
                for y in 0..self.font.glyph_height {
                    for x in self.col as usize * self.font.glyph_width..self.frame.xres() {
                        self.frame.set(x, y + yoff, BLACK);
                    }
                }
            }
            KnownCSI::EraseToCursor | KnownCSI::EraseStartOfLineToCursor => {
                let yoff = self.line as usize * self.font.glyph_height;
                for y in 0..self.font.glyph_height {
                    for x in 0..self.col as usize * self.font.glyph_width {
                        self.frame.set(x, y + yoff, BLACK);
                    }
                }
            }
            KnownCSI::EraseLine => {
                let yoff = self.line as usize * self.font.glyph_height;
                for y in 0..self.font.glyph_height {
                    for x in 0..self.frame.xres() {
                        self.frame.set(x, y + yoff, BLACK);
                    }
                }
            }
            KnownCSI::SetScrollingRegion { top, bottom } => {
                self.scroll_start = top.saturating_sub(1);
                self.scroll_end = bottom.saturating_sub(1).max(self.max_line());
            }
            KnownCSI::SelectGraphicRendition(g) => {
                for g in g {
                    self.handle_select_graphic(g);
                }
            }
            other => println!("{other:?}"),
        }
    }

    fn handle_select_graphic(&mut self, sg: SelectGraphic) {
        match sg {
            SelectGraphic::Reset => {
                self.bg = BLACK;
                self.fg = WHITE;
                self.inverted_c = false;
                self.bold = false;
                self.italic = false;
                self.underline = None;
                self.strike = None;
            }
            SelectGraphic::Bold => self.bold = true,
            SelectGraphic::Italic => self.italic = true,
            SelectGraphic::Underline => self.underline = Some(self.fg),
            SelectGraphic::InvertFgBg => self.inverted_c = true,
            SelectGraphic::NotInvertedFgBg => self.inverted_c = false,
            SelectGraphic::Fg(c) => {
                self.fg = color_code_to_color(c).unwrap_or(WHITE);
            }
            SelectGraphic::Bg(c) => {
                self.bg = color_code_to_color(c).unwrap_or(BLACK);
            }
            other => println!("{other:?}"),
        }
    }
}

pub const WHITE: vga::Color = vga::Color::rgb(255, 255, 255);
pub const BLACK: vga::Color = vga::Color::rgb(0, 0, 0);

fn color_code_to_color(c: Color) -> Option<vga::Color> {
    Some(match c {
        Color::Default => return None,

        Color::Black => vga::Color::rgb(0, 0, 0),
        Color::Red => vga::Color::rgb(170, 0, 0),
        Color::Green => vga::Color::rgb(0, 170, 0),
        Color::Yellow => vga::Color::rgb(170, 80, 0),
        Color::Blue => vga::Color::rgb(0, 0, 170),
        Color::Magenta => vga::Color::rgb(170, 0, 170),
        Color::Cyan => vga::Color::rgb(0, 170, 170),
        Color::White => vga::Color::rgb(192, 192, 192),

        Color::BrightBlack => vga::Color::rgb(170, 170, 170),
        Color::BrightRed => vga::Color::rgb(255, 0, 0),
        Color::BrightGreen => vga::Color::rgb(0, 255, 0),
        Color::BrightYellow => vga::Color::rgb(255, 255, 0),
        Color::BrightBlue => vga::Color::rgb(0, 0, 255),
        Color::BrightMagenta => vga::Color::rgb(255, 0, 170),
        Color::BrightCyan => vga::Color::rgb(0, 255, 255),
        Color::BrightWhite => vga::Color::rgb(255, 255, 255),

        Color::VGA(v) => color_code_to_color(v.as_color())?,

        Color::RGB(RGB { r, g, b }) => vga::Color::rgb(r, g, b),
        _ => return None,
    })
}

pub static FONT: Font = Font {
    data: include_bytes!("../../res/8x13.bin"),
    bold: include_bytes!("../../res/8x13_bold.bin"),
    italic: include_bytes!("../../res/8x13_italic.bin"),
    glyph_per_row: 16,
    glyph_width: 8,
    glyph_height: 13,
};

impl Font {
    pub fn glyph_data(&self, char: char) -> &[u8] {
        let size = (self.glyph_height * self.glyph_width).div_ceil(8);
        let char = char as usize;
        let start = char.clamp(0x20, 0x7E).sub(0x20) * size;
        let end = start + size;
        &self.data[start..end]
    }

    pub fn bold_glyph_data(&self, char: char) -> &[u8] {
        let size = (self.glyph_height * self.glyph_width).div_ceil(8);
        let char = char as usize;
        let start = char.clamp(0x20, 0x7E).sub(0x20) * size;
        let end = start + size;
        &self.bold[start..end]
    }

    pub fn italic_glyph_data(&self, char: char) -> &[u8] {
        let size = (self.glyph_height * self.glyph_width).div_ceil(8);
        let char = char as usize;
        let start = char.clamp(0x20, 0x7E).sub(0x20) * size;
        let end = start + size;
        &self.italic[start..end]
    }
}

pub static mut PARSER: SizedAnsiParser<256> = SizedAnsiParser::new();
pub static mut CONTROLLER: VGAAnsiController = VGAAnsiController::new(vga::framebuffer(), &FONT);

#[allow(static_mut_refs)]
pub fn print(data: &[u8]) {
    for &byte in data {
        unsafe {
            let action = PARSER.next(byte);
            CONTROLLER.advance(action);
        }
    }
}

#[allow(static_mut_refs)]
pub fn update_buffer(frame: FrameBuffer) {
    unsafe {
        CONTROLLER.update_buffer(frame);
    }
}
