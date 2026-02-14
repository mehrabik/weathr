use crate::animation_manager::AnimationManager;
use crate::app_state::AppState;
use crate::config::Config;
use crate::error::WeatherError;
use crate::render::TerminalRenderer;
use crate::scene::WorldScene;
use crate::shell::{key_event_to_bytes, ShellManager};
use crate::weather::{
    create_provider, WeatherClient, WeatherCondition, WeatherData, WeatherLocation,
};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::io;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

const REFRESH_INTERVAL: Duration = Duration::from_secs(300);
const INPUT_POLL_FPS: u64 = 30;
const FRAME_DURATION: Duration = Duration::from_millis(1000 / INPUT_POLL_FPS);

fn generate_offline_weather(rng: &mut impl rand::Rng) -> WeatherData {
    use chrono::{Local, Timelike};
    use rand::RngExt;

    let now = Local::now();
    let hour = now.hour();
    let is_day = (6..18).contains(&hour);

    let conditions = [
        WeatherCondition::Clear,
        WeatherCondition::PartlyCloudy,
        WeatherCondition::Cloudy,
        WeatherCondition::Rain,
    ];

    let condition = conditions[rng.random_range(0..conditions.len())];

    WeatherData {
        condition,
        temperature: rng.random_range(10.0..25.0),
        apparent_temperature: rng.random_range(10.0..25.0),
        humidity: rng.random_range(40.0..80.0),
        precipitation: if condition.is_raining() {
            rng.random_range(1.0..5.0)
        } else {
            0.0
        },
        wind_speed: rng.random_range(5.0..15.0),
        wind_direction: rng.random_range(0.0..360.0),
        cloud_cover: rng.random_range(20.0..80.0),
        pressure: rng.random_range(1000.0..1020.0),
        visibility: Some(10000.0),
        is_day,
        moon_phase: Some(0.5),
        timestamp: now.format("%Y-%m-%dT%H:%M:%S").to_string(),
    }
}

pub struct App {
    state: AppState,
    animations: AnimationManager,
    scene: WorldScene,
    weather_receiver: mpsc::Receiver<Result<WeatherData, WeatherError>>,
    hide_hud: bool,
    provider_name: String,
    shell_manager: Option<ShellManager>,
    background_mode: bool,
    prefix_key_pressed: bool,
}

impl App {
    pub async fn validate_provider(config: &Config) -> Result<(), WeatherError> {
        if config.weather.provider.to_lowercase() == "open_meteo" {
            return Ok(());
        }

        let provider = create_provider(&config.weather)?;

        // Use the user's configured location or a known valid location
        let test_location = WeatherLocation {
            latitude: config.location.latitude,
            longitude: config.location.longitude,
            elevation: None,
        };

        match provider.get_current_weather(&test_location, &config.units).await {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    pub fn new(
        config: &Config,
        simulate_condition: Option<String>,
        simulate_night: bool,
        show_leaves: bool,
        term_width: u16,
        term_height: u16,
    ) -> Result<Self, WeatherError> {
        let location = WeatherLocation {
            latitude: config.location.latitude,
            longitude: config.location.longitude,
            elevation: None,
        };

        let mut state = AppState::new(location, config.location.hide, config.units);
        let mut animations = AnimationManager::new(term_width, term_height, show_leaves);
        let scene = WorldScene::new(term_width, term_height);

        let (tx, rx) = mpsc::channel(1);

        // Set provider name based on config
        let mut provider_name = match config.weather.provider.to_lowercase().as_str() {
            "openweathermap" | "open_weather_map" => String::from("OpenWeatherMap"),
            "weatherapi" | "weather_api" => String::from("WeatherAPI.com"),
            _ => String::from("Open-Meteo.com"),
        };

        if let Some(ref condition_str) = simulate_condition {
            let simulated_condition =
                condition_str
                    .parse::<WeatherCondition>()
                    .unwrap_or_else(|e| {
                        eprintln!("{}", e);
                        WeatherCondition::Clear
                    });

            let weather = WeatherData {
                condition: simulated_condition,
                temperature: 20.0,
                apparent_temperature: 19.0,
                humidity: 65.0,
                precipitation: if simulated_condition.is_raining() {
                    2.5
                } else {
                    0.0
                },
                wind_speed: if simulated_condition.is_thunderstorm() {
                    45.0
                } else {
                    10.0
                },
                wind_direction: 225.0,
                cloud_cover: 50.0,
                pressure: 1013.0,
                visibility: Some(10000.0),
                is_day: !simulate_night,
                moon_phase: Some(0.5),
                timestamp: "simulated".to_string(),
            };

            let rain_intensity = weather.condition.rain_intensity();
            let snow_intensity = weather.condition.snow_intensity();

            let wind_speed = weather.wind_speed;
            let wind_direction = weather.wind_direction;

            state.update_weather(weather);
            animations.update_rain_intensity(rain_intensity);
            animations.update_snow_intensity(snow_intensity);
            animations.update_wind(wind_speed as f32, wind_direction as f32);
        } else {
            let provider = match create_provider(&config.weather) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Error creating weather provider: {}", e);
                    eprintln!("Falling back to Open-Meteo");
                    provider_name = String::from("Open-Meteo.com");
                    Arc::new(crate::weather::OpenMeteoProvider::new())
                }
            };
            let weather_client = WeatherClient::new(provider, REFRESH_INTERVAL);
            let units = config.units;

            tokio::spawn(async move {
                loop {
                    let result = weather_client.get_current_weather(&location, &units).await;
                    if tx.send(result).await.is_err() {
                        break;
                    }
                    tokio::time::sleep(REFRESH_INTERVAL).await;
                }
            });
        }

        // Initialize shell manager if background mode is enabled
        let background_mode = config.shell.background_mode;
        let shell_manager = if background_mode {
            let shell_path = config
                .shell
                .shell_path
                .clone()
                .unwrap_or_else(|| {
                    std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string())
                });
            Some(ShellManager::new(term_width, term_height, &shell_path)?)
        } else {
            None
        };

        Ok(Self {
            state,
            animations,
            scene,
            weather_receiver: rx,
            hide_hud: config.hide_hud,
            provider_name,
            shell_manager,
            background_mode,
            prefix_key_pressed: false,
        })
    }

    pub async fn run(&mut self, renderer: &mut TerminalRenderer) -> io::Result<()> {
        let mut rng = rand::rng();
        loop {
            if let Ok(result) = self.weather_receiver.try_recv() {
                match result {
                    Ok(weather) => {
                        let rain_intensity = weather.condition.rain_intensity();
                        let snow_intensity = weather.condition.snow_intensity();
                        let fog_intensity = weather.condition.fog_intensity();
                        let wind_speed = weather.wind_speed;
                        let wind_direction = weather.wind_direction;

                        self.state.update_weather(weather);
                        self.animations.update_rain_intensity(rain_intensity);
                        self.animations.update_snow_intensity(snow_intensity);
                        self.animations.update_fog_intensity(fog_intensity);
                        self.animations
                            .update_wind(wind_speed as f32, wind_direction as f32);
                    }
                    Err(_error) => {
                        if self.state.current_weather.is_none() {
                            let offline_weather = generate_offline_weather(&mut rng);
                            let rain_intensity = offline_weather.condition.rain_intensity();
                            let snow_intensity = offline_weather.condition.snow_intensity();
                            let fog_intensity = offline_weather.condition.fog_intensity();
                            let wind_speed = offline_weather.wind_speed;
                            let wind_direction = offline_weather.wind_direction;

                            self.state.update_weather(offline_weather);
                            self.state.set_offline_mode(true);
                            self.animations.update_rain_intensity(rain_intensity);
                            self.animations.update_snow_intensity(snow_intensity);
                            self.animations.update_fog_intensity(fog_intensity);
                            self.animations
                                .update_wind(wind_speed as f32, wind_direction as f32);
                        } else {
                            self.state.set_offline_mode(true);
                        }
                    }
                }
            }

            renderer.clear()?;

            let (term_width, term_height) = renderer.get_size();

            self.animations.render_background(
                renderer,
                &self.state.weather_conditions,
                &self.state,
                term_width,
                term_height,
                &mut rng,
            )?;

            self.scene
                .render(renderer, &self.state.weather_conditions)?;

            self.animations.render_chimney_smoke(
                renderer,
                &self.state.weather_conditions,
                term_width,
                term_height,
                &mut rng,
            )?;

            self.animations.render_foreground(
                renderer,
                &self.state.weather_conditions,
                term_width,
                term_height,
                &mut rng,
            )?;

            self.state.update_loading_animation();
            self.state.update_cached_info();

            // Only render HUD and attribution in normal mode (not background)
            if !self.background_mode {
                if !self.hide_hud {
                    renderer.render_line_colored(
                        2,
                        1,
                        &self.state.cached_weather_info,
                        crossterm::style::Color::Cyan,
                    )?;
                }

                let attribution = format!("Weather data by {}", self.provider_name);
                let attribution_x = if term_width > attribution.len() as u16 {
                    term_width - attribution.len() as u16 - 2
                } else {
                    0
                };
                let attribution_y = if term_height > 0 { term_height - 1 } else { 0 };
                renderer.render_line_colored(
                    attribution_x,
                    attribution_y,
                    &attribution,
                    crossterm::style::Color::DarkGrey,
                )?;
            }

            // In background mode, render weather info at the bottom (behind shell)
            if self.background_mode {
                // Get the full weather info and remove the "Press 'q' to quit" part
                let weather_info = self.state.cached_weather_info
                    .replace(" | Press 'q' to quit", "")
                    .replace("Press 'q' to quit", "");

                if !weather_info.is_empty() {
                    let info_x = 2; // Left side of screen
                    let info_y = if term_height > 0 { term_height - 1 } else { 0 };
                    renderer.render_line_colored(
                        info_x,
                        info_y,
                        &weather_info,
                        crossterm::style::Color::Cyan,
                    )?;
                }
            }

            // Render shell overlay if in background mode
            if let Some(ref mut shell) = self.shell_manager {
                // Read all available PTY output (non-blocking loop)
                loop {
                    match shell.read_output() {
                        Ok(output) if !output.is_empty() => {
                            shell.overlay.process_output(&output);
                        }
                        _ => break, // No more data available
                    }
                }

                // Render shell on top of weather
                shell.render(renderer)?;
            }

            renderer.flush()?;

            // Show cursor at shell position after flush
            if let Some(ref shell) = self.shell_manager {
                let (cursor_x, cursor_y) = shell.get_cursor_pos();
                renderer.render_cursor(cursor_x, cursor_y)?;
            }

            if event::poll(FRAME_DURATION)? {
                match event::read()? {
                    Event::Resize(width, height) => {
                        renderer.manual_resize(width, height)?;

                        // Resize shell PTY to match
                        if let Some(ref mut shell) = self.shell_manager {
                            shell.resize(width, height).map_err(|e| {
                                io::Error::new(io::ErrorKind::Other, e.to_string())
                            })?;
                        }
                    }
                    Event::Key(key_event) => {
                        if self.background_mode {
                            if self.handle_background_input(key_event)? {
                                break;
                            }
                        } else {
                            if self.handle_normal_input(key_event) {
                                break;
                            }
                        }
                    }
                    _ => {}
                }
            }

            let (term_width, term_height) = renderer.get_size();
            self.scene.update_size(term_width, term_height);

            self.animations
                .update_sunny_animation(&self.state.weather_conditions);
        }

        Ok(())
    }

    /// Handles input when in background shell mode
    fn handle_background_input(&mut self, key: KeyEvent) -> io::Result<bool> {
        // Check for prefix key (Ctrl-W)
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('w') {
            self.prefix_key_pressed = true;
            return Ok(false);
        }

        // Handle prefix commands
        if self.prefix_key_pressed {
            self.prefix_key_pressed = false;
            match key.code {
                KeyCode::Char('q') => return Ok(true), // Quit
                KeyCode::Char('h') => {
                    // Toggle HUD
                    self.hide_hud = !self.hide_hud;
                    return Ok(false);
                }
                _ => return Ok(false),
            }
        }

        // Forward all other input to shell
        if let Some(ref mut shell) = self.shell_manager {
            let bytes = key_event_to_bytes(key);
            shell.write_input(&bytes)?;
        }

        Ok(false)
    }

    /// Handles input when in normal mode (no shell background)
    fn handle_normal_input(&self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => true,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => true,
            _ => false,
        }
    }
}
