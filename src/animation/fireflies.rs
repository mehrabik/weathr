use crate::render::TerminalRenderer;
use crossterm::style::Color;
use std::io;

struct Firefly {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    glow_phase: f32,
    glow_speed: f32,
    brightness: u8,
}

impl Firefly {
    fn new(terminal_width: u16, terminal_height: u16) -> Self {
        let x = rand::random::<f32>() * terminal_width as f32;
        let y =
            (terminal_height as f32 * 0.5) + (rand::random::<f32>() * terminal_height as f32 * 0.4);

        let vx = (rand::random::<f32>() - 0.5) * 0.3;
        let vy = (rand::random::<f32>() - 0.5) * 0.2;

        let glow_speed = 0.1 + (rand::random::<f32>() * 0.15);
        let glow_phase = rand::random::<f32>() * std::f32::consts::PI * 2.0;

        Self {
            x,
            y,
            vx,
            vy,
            glow_phase,
            glow_speed,
            brightness: 0,
        }
    }

    fn update(&mut self, terminal_width: u16, terminal_height: u16) {
        self.x += self.vx;
        self.y += self.vy;

        if rand::random::<f32>() < 0.02 {
            self.vx = (rand::random::<f32>() - 0.5) * 0.3;
            self.vy = (rand::random::<f32>() - 0.5) * 0.2;
        }

        if self.x < 0.0 {
            self.x = terminal_width as f32;
        } else if self.x > terminal_width as f32 {
            self.x = 0.0;
        }

        if self.y < 0.0 {
            self.y = terminal_height as f32;
        } else if self.y > terminal_height as f32 {
            self.y = 0.0;
        }

        self.glow_phase += self.glow_speed;
        if self.glow_phase > std::f32::consts::PI * 2.0 {
            self.glow_phase -= std::f32::consts::PI * 2.0;
        }

        let glow_value = (self.glow_phase.sin() + 1.0) / 2.0;
        self.brightness = (glow_value * 255.0) as u8;
    }

    fn get_character(&self) -> char {
        if self.brightness > 200 {
            '*'
        } else if self.brightness > 128 {
            '.'
        } else if self.brightness > 64 {
            '·'
        } else {
            ' '
        }
    }

    fn get_color(&self) -> Color {
        if self.brightness > 200 {
            Color::Yellow
        } else if self.brightness > 128 {
            Color::Rgb {
                r: 200,
                g: 255,
                b: 100,
            }
        } else if self.brightness > 64 {
            Color::Rgb {
                r: 150,
                g: 200,
                b: 80,
            }
        } else {
            Color::DarkGrey
        }
    }

    fn is_visible(&self) -> bool {
        self.brightness > 64
    }
}

pub struct FireflySystem {
    fireflies: Vec<Firefly>,
    terminal_width: u16,
    terminal_height: u16,
}

impl FireflySystem {
    pub fn new(terminal_width: u16, terminal_height: u16) -> Self {
        let mut fireflies = Vec::new();
        let count = std::cmp::max(3, terminal_width / 15);

        for _ in 0..count {
            fireflies.push(Firefly::new(terminal_width, terminal_height));
        }

        Self {
            fireflies,
            terminal_width,
            terminal_height,
        }
    }

    pub fn update(&mut self, terminal_width: u16, terminal_height: u16) {
        self.terminal_width = terminal_width;
        self.terminal_height = terminal_height;

        for firefly in &mut self.fireflies {
            firefly.update(terminal_width, terminal_height);
        }

        let target_count = std::cmp::max(3, terminal_width / 15) as usize;
        if self.fireflies.len() < target_count && rand::random::<f32>() < 0.01 {
            self.fireflies
                .push(Firefly::new(terminal_width, terminal_height));
        }
    }

    pub fn render(&self, renderer: &mut TerminalRenderer) -> io::Result<()> {
        for firefly in &self.fireflies {
            if firefly.is_visible() {
                let x = firefly.x as i16;
                let y = firefly.y as i16;

                if x >= 0
                    && y >= 0
                    && x < self.terminal_width as i16
                    && y < self.terminal_height as i16
                {
                    renderer.render_char(
                        x as u16,
                        y as u16,
                        firefly.get_character(),
                        firefly.get_color(),
                    )?;
                }
            }
        }
        Ok(())
    }
}
