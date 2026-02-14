use crate::render::TerminalRenderer;
use crossterm::style::Color;
use std::io;
use vte::{Params, Parser, Perform};

/// Represents a single cell in the shell overlay
#[derive(Clone, Copy, Debug)]
struct Cell {
    character: char,
    fg_color: Color,
    bg_color: Color,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            character: ' ',
            fg_color: Color::Reset,
            bg_color: Color::Reset,
        }
    }
}

/// Internal state for the shell overlay
struct OverlayState {
    cells: Vec<Vec<Cell>>,
    cursor_x: u16,
    cursor_y: u16,
    cursor_visible: bool,
    width: u16,
    height: u16,
    current_fg_color: Color,
    current_bg_color: Color,
    saved_cursor_x: u16,
    saved_cursor_y: u16,
}

/// Manages the shell output buffer and ANSI parsing
pub struct ShellOverlay {
    state: OverlayState,
    parser: Parser,
}

impl ShellOverlay {
    /// Creates a new shell overlay with the specified dimensions
    pub fn new(width: u16, height: u16) -> Self {
        let cells = vec![vec![Cell::default(); width as usize]; height as usize];

        Self {
            state: OverlayState {
                cells,
                cursor_x: 0,
                cursor_y: 0,
                cursor_visible: true,
                width,
                height,
                current_fg_color: Color::Reset,
                current_bg_color: Color::Reset,
                saved_cursor_x: 0,
                saved_cursor_y: 0,
            },
            parser: Parser::new(),
        }
    }

    /// Processes output from the PTY, parsing ANSI escape sequences
    pub fn process_output(&mut self, data: &[u8]) {
        self.parser.advance(&mut self.state, data);
    }

    /// Renders the shell overlay onto the terminal renderer
    pub fn render(&self, renderer: &mut TerminalRenderer) -> io::Result<()> {
        for y in 0..self.state.height {
            for x in 0..self.state.width {
                let cell = &self.state.cells[y as usize][x as usize];

                // Only render non-space characters or cells with explicit background colors
                // This allows weather to show through empty spaces
                if cell.character != ' ' || cell.bg_color != Color::Reset {
                    if cell.bg_color == Color::Reset {
                        // Transparent background - only render character
                        renderer.write_char_transparent(x, y, cell.character, cell.fg_color)?;
                    } else {
                        // Opaque background - render full cell
                        renderer.write_cell(x, y, cell.character, cell.fg_color, cell.bg_color)?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Gets the current cursor position
    pub fn get_cursor_pos(&self) -> (u16, u16) {
        (self.state.cursor_x, self.state.cursor_y)
    }

    /// Resizes the overlay to match new terminal dimensions
    pub fn resize(&mut self, width: u16, height: u16) {
        self.state.width = width;
        self.state.height = height;
        self.state.cells = vec![vec![Cell::default(); width as usize]; height as usize];
        self.state.cursor_x = 0;
        self.state.cursor_y = 0;
    }

}

impl OverlayState {
    /// Writes a character at the current cursor position
    fn write_char(&mut self, c: char) {
        if self.cursor_x < self.width && self.cursor_y < self.height {
            self.cells[self.cursor_y as usize][self.cursor_x as usize] = Cell {
                character: c,
                fg_color: self.current_fg_color,
                bg_color: self.current_bg_color,
            };
        }
    }

    /// Advances the cursor to the next position
    fn advance_cursor(&mut self) {
        self.cursor_x += 1;
        if self.cursor_x >= self.width {
            self.cursor_x = 0;
            self.cursor_y += 1;
            if self.cursor_y >= self.height {
                self.scroll_up();
                self.cursor_y = self.height - 1;
            }
        }
    }

    /// Scrolls the screen up by one line
    fn scroll_up(&mut self) {
        self.cells.remove(0);
        self.cells
            .push(vec![Cell::default(); self.width as usize]);
    }

    /// Clears the screen from cursor to end
    fn clear_to_end(&mut self) {
        // Clear from cursor to end of line
        for x in self.cursor_x..self.width {
            if self.cursor_y < self.height {
                self.cells[self.cursor_y as usize][x as usize] = Cell::default();
            }
        }

        // Clear all lines below cursor
        for y in (self.cursor_y + 1)..self.height {
            for x in 0..self.width {
                self.cells[y as usize][x as usize] = Cell::default();
            }
        }
    }

    /// Clears the entire screen
    fn clear_screen(&mut self) {
        for row in &mut self.cells {
            for cell in row {
                *cell = Cell::default();
            }
        }
    }

    /// Clears the current line from cursor to end
    fn clear_line_to_end(&mut self) {
        if self.cursor_y < self.height {
            for x in self.cursor_x..self.width {
                self.cells[self.cursor_y as usize][x as usize] = Cell::default();
            }
        }
    }

    /// Clears the entire current line
    fn clear_line(&mut self) {
        if self.cursor_y < self.height {
            for x in 0..self.width {
                self.cells[self.cursor_y as usize][x as usize] = Cell::default();
            }
        }
    }

    /// Parses SGR (Select Graphic Rendition) parameters for colors and attributes
    fn parse_sgr(&mut self, params: &Params) {
        let mut iter = params.iter();

        while let Some(param) = iter.next() {
            match param[0] {
                0 => {
                    // Reset all attributes
                    self.current_fg_color = Color::Reset;
                    self.current_bg_color = Color::Reset;
                }
                // Foreground colors (30-37)
                30 => self.current_fg_color = Color::Black,
                31 => self.current_fg_color = Color::Red,
                32 => self.current_fg_color = Color::Green,
                33 => self.current_fg_color = Color::Yellow,
                34 => self.current_fg_color = Color::Blue,
                35 => self.current_fg_color = Color::Magenta,
                36 => self.current_fg_color = Color::Cyan,
                37 => self.current_fg_color = Color::White,
                39 => self.current_fg_color = Color::Reset,

                // Background colors (40-47)
                40 => self.current_bg_color = Color::Black,
                41 => self.current_bg_color = Color::Red,
                42 => self.current_bg_color = Color::Green,
                43 => self.current_bg_color = Color::Yellow,
                44 => self.current_bg_color = Color::Blue,
                45 => self.current_bg_color = Color::Magenta,
                46 => self.current_bg_color = Color::Cyan,
                47 => self.current_bg_color = Color::White,
                49 => self.current_bg_color = Color::Reset,

                // Bright foreground colors (90-97)
                90 => self.current_fg_color = Color::DarkGrey,
                91 => self.current_fg_color = Color::Red,
                92 => self.current_fg_color = Color::Green,
                93 => self.current_fg_color = Color::Yellow,
                94 => self.current_fg_color = Color::Blue,
                95 => self.current_fg_color = Color::Magenta,
                96 => self.current_fg_color = Color::Cyan,
                97 => self.current_fg_color = Color::White,

                // 256-color and RGB color support
                38 => {
                    // Foreground color
                    if let Some(next) = iter.next() {
                        if next[0] == 5 {
                            // 256-color mode
                            if let Some(color_param) = iter.next() {
                                self.current_fg_color = Color::AnsiValue(color_param[0] as u8);
                            }
                        } else if next[0] == 2 {
                            // RGB mode
                            if let (Some(r), Some(g), Some(b)) =
                                (iter.next(), iter.next(), iter.next())
                            {
                                self.current_fg_color =
                                    Color::Rgb { r: r[0] as u8, g: g[0] as u8, b: b[0] as u8 };
                            }
                        }
                    }
                }
                48 => {
                    // Background color
                    if let Some(next) = iter.next() {
                        if next[0] == 5 {
                            // 256-color mode
                            if let Some(color_param) = iter.next() {
                                self.current_bg_color = Color::AnsiValue(color_param[0] as u8);
                            }
                        } else if next[0] == 2 {
                            // RGB mode
                            if let (Some(r), Some(g), Some(b)) =
                                (iter.next(), iter.next(), iter.next())
                            {
                                self.current_bg_color =
                                    Color::Rgb { r: r[0] as u8, g: g[0] as u8, b: b[0] as u8 };
                            }
                        }
                    }
                }

                // Ignore other attributes (bold, italic, etc.) for now
                _ => {}
            }
        }
    }
}

// Implement the VTE Perform trait to handle ANSI escape sequences
impl Perform for OverlayState {
    fn print(&mut self, c: char) {
        self.write_char(c);
        self.advance_cursor();
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            0x08 => {
                // Backspace
                if self.cursor_x > 0 {
                    self.cursor_x -= 1;
                }
            }
            0x09 => {
                // Tab - move to next tab stop (every 8 columns)
                self.cursor_x = ((self.cursor_x / 8) + 1) * 8;
                if self.cursor_x >= self.width {
                    self.cursor_x = 0;
                    self.cursor_y += 1;
                    if self.cursor_y >= self.height {
                        self.scroll_up();
                        self.cursor_y = self.height - 1;
                    }
                }
            }
            0x0A => {
                // Line Feed
                self.cursor_y += 1;
                if self.cursor_y >= self.height {
                    self.scroll_up();
                    self.cursor_y = self.height - 1;
                }
            }
            0x0D => {
                // Carriage Return
                self.cursor_x = 0;
            }
            0x0C => {
                // Form Feed - clear screen
                self.clear_screen();
                self.cursor_x = 0;
                self.cursor_y = 0;
            }
            _ => {}
        }
    }

    fn csi_dispatch(&mut self, params: &Params, _intermediates: &[u8], _ignore: bool, action: char) {
        match action {
            'H' | 'f' => {
                // Cursor Position
                let row = params.iter().next().and_then(|p| p.first()).copied().unwrap_or(1).saturating_sub(1);
                let col = params.iter().nth(1).and_then(|p| p.first()).copied().unwrap_or(1).saturating_sub(1);
                self.cursor_y = (row as u16).min(self.height.saturating_sub(1));
                self.cursor_x = (col as u16).min(self.width.saturating_sub(1));
            }
            'A' => {
                // Cursor Up
                let n = params.iter().next().and_then(|p| p.first()).copied().unwrap_or(1);
                self.cursor_y = self.cursor_y.saturating_sub(n as u16);
            }
            'B' => {
                // Cursor Down
                let n = params.iter().next().and_then(|p| p.first()).copied().unwrap_or(1);
                self.cursor_y = (self.cursor_y + n as u16).min(self.height - 1);
            }
            'C' => {
                // Cursor Forward
                let n = params.iter().next().and_then(|p| p.first()).copied().unwrap_or(1);
                self.cursor_x = (self.cursor_x + n as u16).min(self.width - 1);
            }
            'D' => {
                // Cursor Back
                let n = params.iter().next().and_then(|p| p.first()).copied().unwrap_or(1);
                self.cursor_x = self.cursor_x.saturating_sub(n as u16);
            }
            'J' => {
                // Erase in Display
                let mode = params.iter().next().and_then(|p| p.first()).copied().unwrap_or(0);
                match mode {
                    0 => self.clear_to_end(),
                    1 => {
                        // Clear from beginning to cursor (not implemented)
                    }
                    2 | 3 => {
                        // Clear entire screen
                        self.clear_screen();
                        self.cursor_x = 0;
                        self.cursor_y = 0;
                    }
                    _ => {}
                }
            }
            'K' => {
                // Erase in Line
                let mode = params.iter().next().and_then(|p| p.first()).copied().unwrap_or(0);
                match mode {
                    0 => self.clear_line_to_end(),
                    1 => {
                        // Clear from beginning of line to cursor (not implemented)
                    }
                    2 => self.clear_line(),
                    _ => {}
                }
            }
            'm' => {
                // SGR - Select Graphic Rendition (colors, attributes)
                self.parse_sgr(params);
            }
            'h' => {
                // Set Mode
                if let Some(param) = params.iter().next().and_then(|p| p.first()) {
                    if *param == 25 {
                        // Show cursor
                        self.cursor_visible = true;
                    }
                }
            }
            'l' => {
                // Reset Mode
                if let Some(param) = params.iter().next().and_then(|p| p.first()) {
                    if *param == 25 {
                        // Hide cursor
                        self.cursor_visible = false;
                    }
                }
            }
            's' => {
                // Save cursor position
                self.saved_cursor_x = self.cursor_x;
                self.saved_cursor_y = self.cursor_y;
            }
            'u' => {
                // Restore cursor position
                self.cursor_x = self.saved_cursor_x;
                self.cursor_y = self.saved_cursor_y;
            }
            _ => {
                // Ignore unhandled CSI sequences
            }
        }
    }

    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _action: char) {}

    fn put(&mut self, _byte: u8) {}

    fn unhook(&mut self) {}

    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {}
}
