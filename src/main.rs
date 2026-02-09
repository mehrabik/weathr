mod animation;
mod animation_manager;
mod app;
mod app_state;
mod config;
mod geolocation;
mod render;
mod scene;
mod weather;

use clap::Parser;
use config::Config;
use render::TerminalRenderer;
use std::io;

#[derive(Parser)]
#[command(version, about = "Terminal-based ASCII weather application", long_about = None)]
struct Cli {
    #[arg(
        short,
        long,
        value_name = "CONDITION",
        help = "Simulate weather condition (clear, rain, drizzle, snow, etc.)"
    )]
    simulate: Option<String>,

    #[arg(
        short,
        long,
        help = "Simulate night time (for testing moon, stars, fireflies)"
    )]
    night: bool,

    #[arg(short, long, help = "Enable falling autumn leaves")]
    leaves: bool,

    #[arg(long, help = "Auto-detect location via IP")]
    auto_location: bool,

    #[arg(long, help = "Hide location coordinates in UI")]
    hide_location: bool,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let cli = Cli::parse();

    let mut config = match Config::load() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Error loading config: {}", e);
            eprintln!("\nAuto-detecting location via IP...");
            eprintln!("\nTo customize, create a config file at:");
            eprintln!("  $XDG_CONFIG_HOME/weathr/config.toml");
            eprintln!("  or ~/.config/weathr/config.toml");
            eprintln!("\nExample config.toml:");
            eprintln!("  [location]");
            eprintln!("  latitude = 52.52");
            eprintln!("  longitude = 13.41");
            eprintln!("  auto = false  # Set to true to auto-detect location");
            eprintln!();
            Config::default()
        }
    };

    // CLI Overrides
    if cli.auto_location {
        config.location.auto = true;
    }
    if cli.hide_location {
        config.location.hide = true;
    }

    // Auto-detect location if enabled
    if config.location.auto {
        println!("Auto-detecting location...");
        match geolocation::detect_location().await {
            Ok(geo_loc) => {
                if let Some(city) = &geo_loc.city {
                    println!(
                        "Location detected: {} ({:.4}, {:.4})",
                        city, geo_loc.latitude, geo_loc.longitude
                    );
                } else {
                    println!(
                        "Location detected: {:.4}, {:.4}",
                        geo_loc.latitude, geo_loc.longitude
                    );
                }
                config.location.latitude = geo_loc.latitude;
                config.location.longitude = geo_loc.longitude;
            }
            Err(e) => {
                eprintln!("Failed to auto-detect location: {}", e);
                eprintln!("Using configured/default location.");
            }
        }
    }

    let mut renderer = TerminalRenderer::new()?;
    renderer.init()?;

    let (term_width, term_height) = renderer.get_size();

    let mut app = app::App::new(
        &config,
        cli.simulate,
        cli.night,
        cli.leaves,
        term_width,
        term_height,
    );

    let result = app.run(&mut renderer).await;

    renderer.cleanup()?;

    result
}
