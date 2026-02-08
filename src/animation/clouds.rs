use crate::render::TerminalRenderer;
use crossterm::style::Color;
use std::io;
use std::sync::OnceLock;

static CLOUD_SHAPES: OnceLock<Vec<Vec<String>>> = OnceLock::new();

struct Cloud {
    x: f32,
    y: f32,
    speed: f32,
    shape: Vec<String>,
    color: Color,
}

pub struct CloudSystem {
    clouds: Vec<Cloud>,
    terminal_width: u16,
    terminal_height: u16,
}

impl CloudSystem {
    pub fn new(terminal_width: u16, terminal_height: u16) -> Self {
        let mut clouds = Vec::new();
        // Add a few initial clouds
        let count = std::cmp::max(1, terminal_width / 20);

        for _ in 0..count {
            clouds.push(Self::create_random_cloud(
                terminal_width,
                terminal_height,
                true,
            ));
        }

        Self {
            clouds,
            terminal_width,
            terminal_height,
        }
    }

    fn create_random_cloud(width: u16, height: u16, random_x: bool) -> Cloud {
        let shapes = CLOUD_SHAPES.get_or_init(Self::create_cloud_shapes);

        let shape_idx = (rand::random::<u32>() as usize) % shapes.len();
        let shape = shapes[shape_idx].clone();

        let y_range = height / 3;
        let y = (rand::random::<u16>() % std::cmp::max(1, y_range)) as f32;

        let x = if random_x {
            (rand::random::<u16>() % width) as f32
        } else {
            -(shape[0].len() as f32)
        };

        let speed = 0.05 + (rand::random::<f32>() * 0.1);

        Cloud {
            x,
            y,
            speed,
            shape,
            color: Color::DarkGrey,
        }
    }

    fn create_cloud_shapes() -> Vec<Vec<String>> {
        let shapes = [
            vec![
                "   .--.   ".to_string(),
                " .-(    ). ".to_string(),
                "(___.__)_)".to_string(),
            ],
            vec![
                "      _  _   ".to_string(),
                "    ( `   )_ ".to_string(),
                "   (    )    `)".to_string(),
                "    \\_  (___  )".to_string(),
            ],
            vec![
                "     .--.    ".to_string(),
                "  .-(    ).  ".to_string(),
                " (___.__)__) ".to_string(),
            ],
            vec![
                "   _  _   ".to_string(),
                "  ( `   )_ ".to_string(),
                " (    )   `)".to_string(),
                "  `--'     ".to_string(),
            ],
        ];

        shapes.to_vec()
    }

    pub fn update(&mut self, terminal_width: u16, terminal_height: u16) {
        self.terminal_width = terminal_width;
        self.terminal_height = terminal_height;

        for cloud in &mut self.clouds {
            cloud.x += cloud.speed;
        }

        self.clouds.retain(|c| c.x < terminal_width as f32);
        if self.clouds.len() < (terminal_width / 20) as usize && rand::random::<f32>() < 0.005 {
            self.clouds.push(Self::create_random_cloud(
                terminal_width,
                terminal_height,
                false,
            ));
        }
    }

    pub fn render(&self, renderer: &mut TerminalRenderer) -> io::Result<()> {
        for cloud in &self.clouds {
            for (i, line) in cloud.shape.iter().enumerate() {
                let y = cloud.y as i16 + i as i16;
                let x = cloud.x as i16;

                if y >= 0 && y < self.terminal_height as i16 {
                    renderer.render_line_colored(
                        std::cmp::max(0, x) as u16,
                        y as u16,
                        line,
                        cloud.color,
                    )?;
                }
            }
        }
        Ok(())
    }
}
