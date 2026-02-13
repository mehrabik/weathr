use crate::error::{NetworkError, WeatherError};
use crate::weather::provider::{WeatherProvider, WeatherProviderResponse};
use crate::weather::types::{
    TemperatureUnit, WeatherLocation, WeatherUnits, WindSpeedUnit,
};
use crate::weather::units::{normalize_temperature, normalize_wind_speed};
use async_trait::async_trait;
use serde::Deserialize;
use std::time::Duration;

const OPENWEATHERMAP_BASE_URL: &str = "https://api.openweathermap.org/data/2.5/weather";

pub struct OpenWeatherMapProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
}

#[derive(Debug, Deserialize)]
struct OpenWeatherMapResponse {
    weather: Vec<WeatherDescription>,
    main: MainWeather,
    visibility: Option<i32>,
    wind: Wind,
    clouds: Clouds,
    dt: i64,
    sys: Sys,
}

#[derive(Debug, Deserialize)]
struct WeatherDescription {
    id: i32,
}

#[derive(Debug, Deserialize)]
struct MainWeather {
    temp: f64,
    feels_like: f64,
    pressure: f64,
    humidity: f64,
}

#[derive(Debug, Deserialize)]
struct Wind {
    speed: f64,
    deg: f64,
}

#[derive(Debug, Deserialize)]
struct Clouds {
    all: f64,
}

#[derive(Debug, Deserialize)]
struct Sys {
    sunrise: i64,
    sunset: i64,
}

impl OpenWeatherMapProvider {
    pub fn new(api_key: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|e| {
                eprintln!("Warning: Failed to create custom HTTP client: {}", e);
                eprintln!("Using default client with standard timeout settings.");
                reqwest::Client::new()
            });

        Self {
            client,
            base_url: OPENWEATHERMAP_BASE_URL.to_string(),
            api_key,
        }
    }

    fn temperature_unit_param(unit: &TemperatureUnit) -> &'static str {
        match unit {
            TemperatureUnit::Celsius => "metric",
            TemperatureUnit::Fahrenheit => "imperial",
        }
    }

    fn build_url(&self, location: &WeatherLocation, units: &WeatherUnits) -> String {
        format!(
            "{}?lat={}&lon={}&appid={}&units={}",
            self.base_url,
            location.latitude,
            location.longitude,
            self.api_key,
            Self::temperature_unit_param(&units.temperature)
        )
    }

    fn openweathermap_id_to_wmo_code(id: i32, cloud_cover: f64) -> i32 {
        match id {
            // Clear
            800 => {
                if cloud_cover < 10.0 {
                    0 // Clear sky
                } else {
                    1 // Mainly clear
                }
            }
            // Clouds
            801 => 1,  // Few clouds: Mainly clear
            802 => 2,  // Scattered clouds: Partly cloudy
            803 => 2,  // Broken clouds: Partly cloudy
            804 => 3,  // Overcast clouds
            // Atmosphere
            701 | 721 | 741 => 45, // Mist, Haze, Fog
            // Drizzle
            300..=321 => 51, // Drizzle
            // Rain
            500 => 61,  // Light rain
            501 => 61,  // Moderate rain
            502 => 65,  // Heavy intensity rain
            503 => 65,  // Very heavy rain
            504 => 65,  // Extreme rain
            511 => 66,  // Freezing rain
            520 => 80,  // Light intensity shower rain
            521 => 81,  // Shower rain
            522 => 82,  // Heavy intensity shower rain
            531 => 81,  // Ragged shower rain
            // Snow
            600 => 71,  // Light snow
            601 => 73,  // Snow
            602 => 75,  // Heavy snow
            611 => 77,  // Sleet
            612 => 77,  // Light shower sleet
            613 => 77,  // Shower sleet
            615 => 85,  // Light rain and snow
            616 => 85,  // Rain and snow
            620 => 85,  // Light shower snow
            621 => 85,  // Shower snow
            622 => 86,  // Heavy shower snow
            // Thunderstorm
            200..=202 => 95,  // Thunderstorm with rain
            210..=221 => 95,  // Thunderstorm
            230..=232 => 95,  // Thunderstorm with drizzle
            // Default to clear
            _ => 0,
        }
    }

    fn is_day(current_time: i64, sunrise: i64, sunset: i64) -> i32 {
        if current_time >= sunrise && current_time < sunset {
            1
        } else {
            0
        }
    }

    fn convert_wind_speed(
        speed: f64,
        from_unit: &TemperatureUnit,
        to_unit: &WindSpeedUnit,
    ) -> f64 {
        let speed_ms = match from_unit {
            TemperatureUnit::Celsius => speed,
            TemperatureUnit::Fahrenheit => speed * 0.44704,
        };

        match to_unit {
            WindSpeedUnit::Ms => speed_ms,
            WindSpeedUnit::Kmh => speed_ms * 3.6,
            WindSpeedUnit::Mph => speed_ms * 2.23694,
            WindSpeedUnit::Kn => speed_ms * 1.94384,
        }
    }
}

#[async_trait]
impl WeatherProvider for OpenWeatherMapProvider {
    async fn get_current_weather(
        &self,
        location: &WeatherLocation,
        units: &WeatherUnits,
    ) -> Result<WeatherProviderResponse, WeatherError> {
        let url = self.build_url(location, units);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| WeatherError::Network(NetworkError::from_reqwest(e, &url, 30)))?;

        let data: OpenWeatherMapResponse = response
            .json()
            .await
            .map_err(|e| WeatherError::Network(NetworkError::from_reqwest(e, &url, 30)))?;

        let weather_id = data.weather.first().map(|w| w.id).unwrap_or(800);
        let weather_code = Self::openweathermap_id_to_wmo_code(weather_id, data.clouds.all);
        let is_day = Self::is_day(data.dt, data.sys.sunrise, data.sys.sunset);

        let wind_speed = Self::convert_wind_speed(
            data.wind.speed,
            &units.temperature,
            &units.wind_speed,
        );

        let moon_phase = Some(0.5);

        let visibility_meters = data.visibility.map(|v| v as f64);

        Ok(WeatherProviderResponse {
            weather_code,
            temperature: normalize_temperature(data.main.temp, units.temperature),
            apparent_temperature: normalize_temperature(data.main.feels_like, units.temperature),
            humidity: data.main.humidity,
            precipitation: 0.0,
            wind_speed: normalize_wind_speed(wind_speed, units.wind_speed),
            wind_direction: data.wind.deg,
            cloud_cover: data.clouds.all,
            pressure: data.main.pressure,
            visibility: visibility_meters,
            is_day,
            moon_phase,
            timestamp: chrono::DateTime::from_timestamp(data.dt, 0)
                .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S").to_string())
                .unwrap_or_else(|| "unknown".to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openweathermap_to_wmo_mapping() {
        assert_eq!(OpenWeatherMapProvider::openweathermap_id_to_wmo_code(800, 5.0), 0);
        assert_eq!(OpenWeatherMapProvider::openweathermap_id_to_wmo_code(800, 15.0), 1);
        assert_eq!(OpenWeatherMapProvider::openweathermap_id_to_wmo_code(801, 0.0), 1);
        assert_eq!(OpenWeatherMapProvider::openweathermap_id_to_wmo_code(802, 0.0), 2);
        assert_eq!(OpenWeatherMapProvider::openweathermap_id_to_wmo_code(804, 0.0), 3);
        assert_eq!(OpenWeatherMapProvider::openweathermap_id_to_wmo_code(741, 0.0), 45);
        assert_eq!(OpenWeatherMapProvider::openweathermap_id_to_wmo_code(300, 0.0), 51);
        assert_eq!(OpenWeatherMapProvider::openweathermap_id_to_wmo_code(500, 0.0), 61);
        assert_eq!(OpenWeatherMapProvider::openweathermap_id_to_wmo_code(600, 0.0), 71);
        assert_eq!(OpenWeatherMapProvider::openweathermap_id_to_wmo_code(200, 0.0), 95);
    }

    #[test]
    fn test_is_day() {
        assert_eq!(OpenWeatherMapProvider::is_day(1000, 900, 1800), 1);
        assert_eq!(OpenWeatherMapProvider::is_day(500, 900, 1800), 0);
        assert_eq!(OpenWeatherMapProvider::is_day(2000, 900, 1800), 0);
    }
}
