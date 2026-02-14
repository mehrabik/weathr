use crate::error::{NetworkError, WeatherError};
use crate::weather::provider::{WeatherProvider, WeatherProviderResponse};
use crate::weather::types::{
    PrecipitationUnit, TemperatureUnit, WeatherLocation, WeatherUnits, WindSpeedUnit,
};
use crate::weather::units::{normalize_precipitation, normalize_temperature, normalize_wind_speed};
use async_trait::async_trait;
use serde::Deserialize;
use std::time::Duration;

const WEATHERAPI_BASE_URL: &str = "https://api.weatherapi.com/v1/current.json";

pub struct WeatherApiProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
}

#[derive(Debug, Deserialize)]
struct WeatherApiResponse {
    current: CurrentWeather,
}

#[derive(Debug, Deserialize)]
struct CurrentWeather {
    last_updated: String,
    temp_c: f64,
    temp_f: f64,
    is_day: i32,
    condition: Condition,
    wind_mph: f64,
    wind_kph: f64,
    wind_degree: f64,
    pressure_mb: f64,
    precip_mm: f64,
    precip_in: f64,
    humidity: f64,
    cloud: f64,
    feelslike_c: f64,
    feelslike_f: f64,
    vis_km: f64,
}

#[derive(Debug, Deserialize)]
struct Condition {
    code: i32,
}

impl WeatherApiProvider {
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
            base_url: WEATHERAPI_BASE_URL.to_string(),
            api_key,
        }
    }

    fn build_url(&self, location: &WeatherLocation) -> String {
        format!(
            "{}?key={}&q={},{}&aqi=no",
            self.base_url, self.api_key, location.latitude, location.longitude
        )
    }

    fn weatherapi_code_to_wmo_code(code: i32) -> i32 {
        match code {
            // Sunny/Clear
            1000 => 0, // Clear
            // Partly cloudy
            1003 => 1, // Mainly clear
            // Cloudy
            1006 => 2, // Partly cloudy
            1009 => 3, // Overcast
            // Fog/Mist
            1030 | 1135 | 1147 => 45, // Fog
            // Drizzle
            1063 | 1150 | 1153 | 1168 | 1171 => 51, // Drizzle
            // Patchy rain
            1180 | 1183 | 1186 => 61, // Light to moderate rain
            // Moderate to heavy rain
            1189 | 1192 | 1195 => 63, // Moderate to heavy rain
            // Freezing rain
            1198 | 1201 | 1204 | 1207 | 1237 | 1261 => 66, // Freezing rain
            // Light snow
            1066 | 1210 | 1213 | 1216 | 1255 => 71, // Light snow
            // Moderate snow
            1219 | 1222 | 1258 => 73, // Moderate snow
            // Heavy snow
            1225 | 1282 => 75, // Heavy snow
            // Snow grains/Ice pellets
            1069 | 1072 | 1114 | 1117 | 1249 | 1252 => 77, // Snow grains
            // Rain showers
            1240 | 1243 | 1246 => 80, // Rain showers
            // Snow showers
            1279 => 85, // Snow showers
            // Thunderstorm
            1087 | 1273 => 95, // Thunderstorm
            // Thunderstorm with hail
            1264 | 1276 => 99, // Thunderstorm with hail
            // Default
            _ => 0,
        }
    }

    fn get_temperature(current: &CurrentWeather, unit: &TemperatureUnit) -> f64 {
        match unit {
            TemperatureUnit::Celsius => current.temp_c,
            TemperatureUnit::Fahrenheit => current.temp_f,
        }
    }

    fn get_feels_like(current: &CurrentWeather, unit: &TemperatureUnit) -> f64 {
        match unit {
            TemperatureUnit::Celsius => current.feelslike_c,
            TemperatureUnit::Fahrenheit => current.feelslike_f,
        }
    }

    fn get_wind_speed(current: &CurrentWeather, unit: &WindSpeedUnit) -> f64 {
        match unit {
            WindSpeedUnit::Kmh => current.wind_kph,
            WindSpeedUnit::Mph => current.wind_mph,
            WindSpeedUnit::Ms => current.wind_kph / 3.6,
            WindSpeedUnit::Kn => current.wind_kph / 1.852,
        }
    }

    fn get_precipitation(current: &CurrentWeather, unit: &PrecipitationUnit) -> f64 {
        match unit {
            PrecipitationUnit::Mm => current.precip_mm,
            PrecipitationUnit::Inch => current.precip_in,
        }
    }
}

#[async_trait]
impl WeatherProvider for WeatherApiProvider {
    async fn get_current_weather(
        &self,
        location: &WeatherLocation,
        units: &WeatherUnits,
    ) -> Result<WeatherProviderResponse, WeatherError> {
        let url = self.build_url(location);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| WeatherError::Network(NetworkError::from_reqwest(e, &url, 30)))?;

        let data: WeatherApiResponse = response
            .json()
            .await
            .map_err(|e| WeatherError::Network(NetworkError::from_reqwest(e, &url, 30)))?;

        let weather_code = Self::weatherapi_code_to_wmo_code(data.current.condition.code);
        let temperature = Self::get_temperature(&data.current, &units.temperature);
        let feels_like = Self::get_feels_like(&data.current, &units.temperature);
        let wind_speed = Self::get_wind_speed(&data.current, &units.wind_speed);
        let precipitation = Self::get_precipitation(&data.current, &units.precipitation);

        let moon_phase = Some(0.5);

        let visibility_meters = Some(data.current.vis_km * 1000.0);

        Ok(WeatherProviderResponse {
            weather_code,
            temperature: normalize_temperature(temperature, units.temperature),
            apparent_temperature: normalize_temperature(feels_like, units.temperature),
            humidity: data.current.humidity,
            precipitation: normalize_precipitation(precipitation, units.precipitation),
            wind_speed: normalize_wind_speed(wind_speed, units.wind_speed),
            wind_direction: data.current.wind_degree,
            cloud_cover: data.current.cloud,
            pressure: data.current.pressure_mb,
            visibility: visibility_meters,
            is_day: data.current.is_day,
            moon_phase,
            timestamp: data.current.last_updated,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weatherapi_to_wmo_mapping() {
        assert_eq!(WeatherApiProvider::weatherapi_code_to_wmo_code(1000), 0);
        assert_eq!(WeatherApiProvider::weatherapi_code_to_wmo_code(1003), 1);
        assert_eq!(WeatherApiProvider::weatherapi_code_to_wmo_code(1006), 2);
        assert_eq!(WeatherApiProvider::weatherapi_code_to_wmo_code(1009), 3);
        assert_eq!(WeatherApiProvider::weatherapi_code_to_wmo_code(1030), 45);
        assert_eq!(WeatherApiProvider::weatherapi_code_to_wmo_code(1150), 51);
        assert_eq!(WeatherApiProvider::weatherapi_code_to_wmo_code(1180), 61);
        assert_eq!(WeatherApiProvider::weatherapi_code_to_wmo_code(1210), 71);
        assert_eq!(WeatherApiProvider::weatherapi_code_to_wmo_code(1087), 95);
    }
}
