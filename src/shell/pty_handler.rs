use crate::error::ShellError;
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::io::{self, Read, Write};
use tokio::sync::mpsc;

/// Handles the pseudo-terminal and shell process lifecycle
pub struct PtyHandler {
    _pty_master: Box<dyn MasterPty + Send>,
    _shell_child: Box<dyn Child + Send>,
    output_receiver: mpsc::UnboundedReceiver<Vec<u8>>,
    writer: Box<dyn Write + Send>,
}

impl PtyHandler {
    /// Creates a new PTY handler with a shell process
    pub fn new(width: u16, height: u16, shell_path: &str) -> Result<Self, ShellError> {
        let pty_system = native_pty_system();

        // Create PTY with specified dimensions
        let pty_pair = pty_system
            .openpty(PtySize {
                rows: height,
                cols: width,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| ShellError::PtyCreation(e.to_string()))?;

        // Build the shell command with -i and -l flags for interactive login shell
        let mut cmd = CommandBuilder::new(shell_path);
        cmd.arg("-i"); // Force interactive mode
        cmd.arg("-l"); // Make it a login shell (loads profile, enables history)
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        cmd.env("PS1", "$ "); // Simple prompt to avoid complex shell configurations
        cmd.env("PROMPT_EOL_MARK", ""); // Disable zsh's partial line indicator (%)
        cmd.env("PROMPT_SP", ""); // Disable zsh's special prompt spacing

        // Preserve essential environment variables
        if let Ok(home) = std::env::var("HOME") {
            cmd.env("HOME", home);
        }
        if let Ok(user) = std::env::var("USER") {
            cmd.env("USER", user);
        }
        if let Ok(path) = std::env::var("PATH") {
            cmd.env("PATH", path);
        }
        if let Ok(shell) = std::env::var("SHELL") {
            cmd.env("SHELL", shell);
        }

        // Preserve shell history configuration
        if let Ok(histfile) = std::env::var("HISTFILE") {
            cmd.env("HISTFILE", histfile);
        }
        if let Ok(histsize) = std::env::var("HISTSIZE") {
            cmd.env("HISTSIZE", histsize);
        }
        if let Ok(savehist) = std::env::var("SAVEHIST") {
            cmd.env("SAVEHIST", savehist);
        }
        // zsh history options
        if let Ok(histfile) = std::env::var("ZDOTDIR") {
            cmd.env("ZDOTDIR", histfile);
        }

        // Spawn the shell process
        let child = pty_pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| ShellError::ShellSpawn(e.to_string()))?;

        // Close slave end in parent process
        drop(pty_pair.slave);

        // Get reader and writer for the PTY
        let mut reader = pty_pair
            .master
            .try_clone_reader()
            .map_err(|e| ShellError::PtyCreation(e.to_string()))?;

        let writer = pty_pair
            .master
            .take_writer()
            .map_err(|e| ShellError::PtyCreation(e.to_string()))?;

        // Create channel for async PTY output
        let (tx, rx) = mpsc::unbounded_channel();

        // Create a barrier to ensure thread starts properly
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let barrier_clone = barrier.clone();

        // Spawn background task to read PTY output
        std::thread::spawn(move || {
            // Signal that thread has started
            barrier_clone.wait();

            let mut buf = vec![0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break; // Channel closed
                        }
                    }
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(_) => break,
                }
            }
        });

        // Wait for thread to start
        barrier.wait();

        Ok(Self {
            _pty_master: pty_pair.master,
            _shell_child: child,
            output_receiver: rx,
            writer: Box::new(writer),
        })
    }

    /// Reads output from the PTY (non-blocking)
    pub fn read_output(&mut self) -> io::Result<Vec<u8>> {
        // Try to receive data from the channel (non-blocking)
        match self.output_receiver.try_recv() {
            Ok(data) => Ok(data),
            Err(mpsc::error::TryRecvError::Empty) => Ok(vec![]), // No data available right now
            Err(mpsc::error::TryRecvError::Disconnected) => {
                // Channel closed - shell has exited
                Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "Shell process has exited",
                ))
            }
        }
    }

    /// Writes input to the PTY
    pub fn write_input(&mut self, data: &[u8]) -> io::Result<()> {
        self.writer.write_all(data)?;
        self.writer.flush()
    }

    /// Resizes the PTY to match new terminal dimensions
    pub fn resize(&mut self, width: u16, height: u16) -> Result<(), ShellError> {
        self._pty_master
            .resize(PtySize {
                rows: height,
                cols: width,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| ShellError::PtyCreation(format!("Failed to resize PTY: {}", e)))?;

        Ok(())
    }

    /// Sends exit command to the shell to allow it to save history
    pub fn send_exit(&mut self) -> io::Result<()> {
        // Send exit command followed by newline
        let _ = self.write_input(b"exit\n");
        // Give shell a moment to process and save history
        std::thread::sleep(std::time::Duration::from_millis(50));
        Ok(())
    }
}

impl Drop for PtyHandler {
    fn drop(&mut self) {
        // Try to send exit command to allow shell to save history
        let _ = self.send_exit();
    }
}
