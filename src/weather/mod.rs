pub mod client;
pub mod factory;
pub mod normalizer;
pub mod open_meteo;
pub mod openweathermap;
pub mod provider;
pub mod types;
pub mod units;
pub mod weatherapi;

pub use client::WeatherClient;
pub use factory::create_provider;
pub use open_meteo::OpenMeteoProvider;
pub use types::{
    FogIntensity, RainIntensity, SnowIntensity, WeatherCondition, WeatherConditions, WeatherData,
    WeatherLocation, WeatherUnits,
};
pub use units::{format_precipitation, format_temperature, format_wind_speed};
