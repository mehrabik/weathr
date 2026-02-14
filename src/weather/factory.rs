use crate::config::WeatherConfig;
use crate::error::WeatherError;
use crate::weather::open_meteo::OpenMeteoProvider;
use crate::weather::openweathermap::OpenWeatherMapProvider;
use crate::weather::provider::WeatherProvider;
use crate::weather::weatherapi::WeatherApiProvider;
use std::sync::Arc;

pub fn create_provider(config: &WeatherConfig) -> Result<Arc<dyn WeatherProvider>, WeatherError> {
    match config.provider.to_lowercase().as_str() {
        "open_meteo" | "openmeteo" => Ok(Arc::new(OpenMeteoProvider::new())),
        "openweathermap" | "open_weather_map" => {
            let api_key = config.api_key.clone().ok_or_else(|| {
                WeatherError::Configuration(
                    "OpenWeatherMap requires an API key. Add 'api_key' to the [weather] section in your config.toml".to_string(),
                )
            })?;
            Ok(Arc::new(OpenWeatherMapProvider::new(api_key)))
        }
        "weatherapi" | "weather_api" => {
            let api_key = config.api_key.clone().ok_or_else(|| {
                WeatherError::Configuration(
                    "WeatherAPI requires an API key. Add 'api_key' to the [weather] section in your config.toml".to_string(),
                )
            })?;
            Ok(Arc::new(WeatherApiProvider::new(api_key)))
        }
        _ => Err(WeatherError::Configuration(format!(
            "Unknown weather provider: '{}'. Valid options: open_meteo, openweathermap, weatherapi",
            config.provider
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_open_meteo_provider() {
        let config = WeatherConfig {
            provider: "open_meteo".to_string(),
            api_key: None,
        };
        let result = create_provider(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_openweathermap_provider_without_key() {
        let config = WeatherConfig {
            provider: "openweathermap".to_string(),
            api_key: None,
        };
        let result = create_provider(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_openweathermap_provider_with_key() {
        let config = WeatherConfig {
            provider: "openweathermap".to_string(),
            api_key: Some("test_key".to_string()),
        };
        let result = create_provider(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_weatherapi_provider_without_key() {
        let config = WeatherConfig {
            provider: "weatherapi".to_string(),
            api_key: None,
        };
        let result = create_provider(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_weatherapi_provider_with_key() {
        let config = WeatherConfig {
            provider: "weatherapi".to_string(),
            api_key: Some("test_key".to_string()),
        };
        let result = create_provider(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_unknown_provider() {
        let config = WeatherConfig {
            provider: "unknown_provider".to_string(),
            api_key: None,
        };
        let result = create_provider(&config);
        assert!(result.is_err());
    }
}
