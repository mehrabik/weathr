mod input;
mod overlay;
mod pty_handler;

pub use input::key_event_to_bytes;
use overlay::ShellOverlay;
use pty_handler::PtyHandler;

use crate::error::ShellError;
use crate::render::TerminalRenderer;
use std::io;

/// Manages the shell process and its integration with the weather display
pub struct ShellManager {
    pty_handler: PtyHandler,
    pub overlay: ShellOverlay,
}

impl ShellManager {
    /// Creates a new shell manager with the specified dimensions and shell path
    pub fn new(width: u16, height: u16, shell_path: &str) -> Result<Self, ShellError> {
        let mut pty_handler = PtyHandler::new(width, height, shell_path)?;
        let overlay = ShellOverlay::new(width, height);

        // Give the shell a moment to initialize
        std::thread::sleep(std::time::Duration::from_millis(100));

        // For zsh, disable prompt spacing features that add extra newlines
        // These setopt commands work more reliably than environment variables
        let _ = pty_handler.write_input(b"setopt nopromptsp 2>/dev/null; setopt nopromptcr 2>/dev/null; clear\r");

        Ok(Self {
            pty_handler,
            overlay,
        })
    }

    /// Reads output from the PTY (non-blocking)
    pub fn read_output(&mut self) -> io::Result<Vec<u8>> {
        self.pty_handler.read_output()
    }

    /// Writes input to the PTY
    pub fn write_input(&mut self, data: &[u8]) -> io::Result<()> {
        self.pty_handler.write_input(data)
    }

    /// Resizes the PTY and overlay to match new terminal dimensions
    pub fn resize(&mut self, width: u16, height: u16) -> Result<(), ShellError> {
        self.pty_handler.resize(width, height)?;
        self.overlay.resize(width, height);
        Ok(())
    }

    /// Renders the shell overlay onto the terminal renderer
    pub fn render(&self, renderer: &mut TerminalRenderer) -> io::Result<()> {
        self.overlay.render(renderer)
    }

    /// Gets the current cursor position from the shell overlay
    pub fn get_cursor_pos(&self) -> (u16, u16) {
        self.overlay.get_cursor_pos()
    }
}
