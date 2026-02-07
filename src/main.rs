mod animation;
mod config;
mod display;
mod render;
mod scene;
mod weather;

use animation::{
    birds::BirdSystem, clouds::CloudSystem, moon::MoonSystem, raindrops::RaindropSystem,
    snow::SnowSystem, stars::StarSystem, sunny::SunnyAnimation, thunderstorm::ThunderstormSystem,
    AnimationController,
};
use clap::Parser;
use config::Config;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use render::TerminalRenderer;
use scene::WorldScene;
use std::io;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use weather::{
    OpenMeteoProvider, RainIntensity, SnowIntensity, WeatherClient, WeatherCondition, WeatherData,
    WeatherLocation, WeatherUnits,
};

const REFRESH_INTERVAL: Duration = Duration::from_secs(300);
const FRAME_DELAY: Duration = Duration::from_millis(500);
const TARGET_FPS: u64 = 30;
const FRAME_DURATION: Duration = Duration::from_millis(1000 / TARGET_FPS);

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
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let cli = Cli::parse();

    let config = match Config::load() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Error loading config: {}", e);
            eprintln!("\nContinuing with default location (Berlin: 52.52°N, 13.41°E)");
            eprintln!("\nTo customize, create a config file at:");
            eprintln!("  $XDG_CONFIG_HOME/weathr/config.toml");
            eprintln!("  or ~/.config/weathr/config.toml");
            eprintln!("\nExample config.toml:");
            eprintln!("  [location]");
            eprintln!("  latitude = 52.52");
            eprintln!("  longitude = 13.41");
            eprintln!();
            Config::default()
        }
    };

    let mut renderer = TerminalRenderer::new()?;
    renderer.init()?;

    let result = run_app(&config, &mut renderer, cli.simulate).await;

    renderer.cleanup()?;

    result
}

async fn run_app(
    config: &Config,
    renderer: &mut TerminalRenderer,
    simulate_condition: Option<String>,
) -> io::Result<()> {
    let mut world_scene = WorldScene::new(0, 0); // Will update size later
    let sunny_animation = SunnyAnimation::new();
    let mut animation_controller = AnimationController::new();

    let provider = Arc::new(OpenMeteoProvider::new());
    let weather_client = WeatherClient::new(provider, REFRESH_INTERVAL);

    let location = WeatherLocation {
        latitude: config.location.latitude,
        longitude: config.location.longitude,
        elevation: None,
    };
    let units = WeatherUnits::default();

    let (tx, mut rx) = mpsc::channel(1);

    if simulate_condition.is_none() {
        let client = weather_client.clone();
        let loc = location.clone();
        let u = units.clone();

        tokio::spawn(async move {
            loop {
                let result = client.get_current_weather(&loc, &u).await;
                if tx.send(result).await.is_err() {
                    break;
                }
                tokio::time::sleep(REFRESH_INTERVAL).await;
            }
        });
    }

    let mut last_frame_time = Instant::now();
    let mut current_weather = None;
    let mut weather_error: Option<String> = None;
    let mut is_raining = false;
    let mut is_snowing = false;
    let mut is_thunderstorm = false;
    let mut is_cloudy = false;
    let mut is_day = true;

    let mut loading_frame = 0;
    let loading_chars = ['|', '/', '-', '\\'];
    let mut last_loading_update = Instant::now();

    let (term_width, term_height) = renderer.get_size();
    world_scene.update_size(term_width, term_height);
    let mut raindrop_system = RaindropSystem::new(term_width, term_height, RainIntensity::Light);
    let mut snow_system = SnowSystem::new(term_width, term_height, SnowIntensity::Light);
    let mut thunderstorm_system = ThunderstormSystem::new(term_width, term_height);
    let mut cloud_system = CloudSystem::new(term_width, term_height);
    let mut bird_system = BirdSystem::new(term_width, term_height);
    let mut star_system = StarSystem::new(term_width, term_height);
    let mut moon_system = MoonSystem::new(term_width, term_height);

    if let Some(ref condition_str) = simulate_condition {
        let simulated_condition = parse_weather_condition(condition_str);
        is_thunderstorm = simulated_condition.is_thunderstorm();
        is_snowing = simulated_condition.is_snowing();
        is_raining = simulated_condition.is_raining() && !is_thunderstorm;

        raindrop_system.set_intensity(simulated_condition.rain_intensity());
        snow_system.set_intensity(simulated_condition.snow_intensity());
        is_cloudy = simulated_condition.is_cloudy();

        is_day = true;

        current_weather = Some(WeatherData {
            condition: simulated_condition,
            temperature: 20.0,
            apparent_temperature: 19.0,
            humidity: 65.0,
            precipitation: if simulated_condition.is_raining() {
                2.5
            } else {
                0.0
            },
            wind_speed: 10.0,
            wind_direction: 180.0,
            cloud_cover: 50.0,
            pressure: 1013.0,
            visibility: Some(10000.0),
            is_day: true,
            moon_phase: Some(0.5), // Simulated Full Moon
            timestamp: "simulated".to_string(),
        });
    }

    loop {
        if let Ok(result) = rx.try_recv() {
            match result {
                Ok(weather) => {
                    is_thunderstorm = weather.condition.is_thunderstorm();
                    is_snowing = weather.condition.is_snowing();
                    is_raining = weather.condition.is_raining() && !is_thunderstorm;

                    raindrop_system.set_intensity(weather.condition.rain_intensity());
                    snow_system.set_intensity(weather.condition.snow_intensity());
                    is_cloudy = weather.condition.is_cloudy();
                    is_day = weather.is_day;

                    if let Some(phase) = weather.moon_phase {
                        moon_system.set_phase(phase);
                    }

                    current_weather = Some(weather);
                    weather_error = None;
                }
                Err(e) => {
                    weather_error = Some(format!("Error fetching weather: {}", e));
                }
            }
        }

        renderer.update_size()?;
        let (term_width, term_height) = renderer.get_size();
        world_scene.update_size(term_width, term_height);

        renderer.clear()?;

        let condition_text = if let Some(ref weather) = current_weather {
            match weather.condition {
                WeatherCondition::Clear => "Clear",
                WeatherCondition::PartlyCloudy => "Partly Cloudy",
                WeatherCondition::Cloudy => "Cloudy",
                WeatherCondition::Overcast => "Overcast",
                WeatherCondition::Fog => "Fog",
                WeatherCondition::Drizzle => "Drizzle",
                WeatherCondition::Rain => "Rain",
                WeatherCondition::FreezingRain => "Freezing Rain",
                WeatherCondition::Snow => "Snow",
                WeatherCondition::SnowGrains => "Snow Grains",
                WeatherCondition::RainShowers => "Rain Showers",
                WeatherCondition::SnowShowers => "Snow Showers",
                WeatherCondition::Thunderstorm => "Thunderstorm",
                WeatherCondition::ThunderstormHail => "Thunderstorm with Hail",
            }
        } else {
            // Update loading frame
            if last_loading_update.elapsed() >= Duration::from_millis(100) {
                loading_frame = (loading_frame + 1) % loading_chars.len();
                last_loading_update = Instant::now();
            }
            "Loading"
        };

        let weather_info = if let Some(ref error) = weather_error {
            format!(
                "{} | Location: {:.2}°N, {:.2}°E | Press 'q' to quit",
                error, location.latitude, location.longitude
            )
        } else if let Some(ref weather) = current_weather {
            format!(
                "Weather: {} | Temp: {:.1}°C | Location: {:.2}°N, {:.2}°E | Press 'q' to quit",
                condition_text, weather.temperature, location.latitude, location.longitude
            )
        } else {
            format!(
                "Weather: Loading... {} | Location: {:.2}°N, {:.2}°E | Press 'q' to quit",
                loading_chars[loading_frame], location.latitude, location.longitude
            )
        };

        renderer.render_line_colored(2, 1, &weather_info, crossterm::style::Color::Cyan)?;

        if !is_day {
            star_system.update(term_width, term_height);
            star_system.render(renderer)?;
            moon_system.update(term_width, term_height);
            moon_system.render(renderer)?;
        }

        if is_cloudy || (!is_raining && !is_thunderstorm && !is_snowing) {
            if is_cloudy {
                cloud_system.update(term_width, term_height);
                cloud_system.render(renderer)?;
            }

            if !is_raining && !is_thunderstorm && !is_snowing && is_day {
                bird_system.update(term_width, term_height);
                bird_system.render(renderer)?;
            }
        }

        let show_sun = if is_day {
            if let Some(ref weather) = current_weather {
                matches!(
                    weather.condition,
                    WeatherCondition::Clear | WeatherCondition::PartlyCloudy
                )
            } else {
                false
            }
        } else {
            false
        };

        if show_sun && !is_raining && !is_thunderstorm && !is_snowing {
            let animation_y = if term_height > 20 { 3 } else { 2 };
            animation_controller.render_frame(renderer, &sunny_animation, animation_y)?;
        }

        // Render World Scene (House, Ground, Decorations)
        world_scene.render(renderer)?;

        // Render foreground (rain/thunder)
        // Thunderstorm includes rain + lightning
        if is_thunderstorm {
            // Update and render rain first
            raindrop_system.update(term_width, term_height);
            raindrop_system.render(renderer)?;

            // Then lightning
            thunderstorm_system.update(term_width, term_height);
            thunderstorm_system.render(renderer)?;

            // Check for flash
            if thunderstorm_system.is_flashing() {
                renderer.flash_screen()?;
            }
        } else if is_raining {
            raindrop_system.update(term_width, term_height);
            raindrop_system.render(renderer)?;
        } else if is_snowing {
            snow_system.update(term_width, term_height);
            snow_system.render(renderer)?;
        }

        renderer.flush()?;

        if event::poll(FRAME_DURATION)? {
            if let Event::Key(key_event) = event::read()? {
                match key_event.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => break,
                    KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                        break
                    }
                    _ => {}
                }
            }
        }

        if !is_raining && !is_thunderstorm && !is_snowing {
            // Update sunny animation frame less frequently
            if last_frame_time.elapsed() >= FRAME_DELAY {
                animation_controller.next_frame(&sunny_animation);
                last_frame_time = Instant::now();
            }
        }
    }

    Ok(())
}

fn parse_weather_condition(input: &str) -> WeatherCondition {
    match input.to_lowercase().as_str() {
        "clear" | "sunny" => WeatherCondition::Clear,
        "partly-cloudy" | "partly_cloudy" | "partlycloudy" => WeatherCondition::PartlyCloudy,
        "cloudy" => WeatherCondition::Cloudy,
        "overcast" => WeatherCondition::Overcast,
        "fog" | "foggy" => WeatherCondition::Fog,
        "drizzle" => WeatherCondition::Drizzle,
        "rain" | "rainy" => WeatherCondition::Rain,
        "freezing-rain" | "freezing_rain" | "freezingrain" => WeatherCondition::FreezingRain,
        "snow" | "snowy" => WeatherCondition::Snow,
        "snow-grains" | "snow_grains" | "snowgrains" => WeatherCondition::SnowGrains,
        "rain-showers" | "rain_showers" | "rainshowers" | "showers" => {
            WeatherCondition::RainShowers
        }
        "snow-showers" | "snow_showers" | "snowshowers" => WeatherCondition::SnowShowers,
        "thunderstorm" | "thunder" => WeatherCondition::Thunderstorm,
        "thunderstorm-hail" | "thunderstorm_hail" | "hail" => WeatherCondition::ThunderstormHail,
        _ => {
            eprintln!("Unknown weather condition '{}', defaulting to Clear", input);
            WeatherCondition::Clear
        }
    }
}
