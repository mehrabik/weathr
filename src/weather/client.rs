use crate::weather::normalizer::WeatherNormalizer;
use crate::weather::provider::WeatherProvider;
use crate::weather::types::{WeatherData, WeatherLocation, WeatherUnits};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct WeatherClient {
    provider: Arc<dyn WeatherProvider>,
    cache: Arc<RwLock<Option<CachedWeather>>>,
    cache_duration: Duration,
}

struct CachedWeather {
    data: WeatherData,
    fetched_at: Instant,
}

impl WeatherClient {
    pub fn new(provider: Arc<dyn WeatherProvider>, cache_duration: Duration) -> Self {
        Self {
            provider,
            cache: Arc::new(RwLock::new(None)),
            cache_duration,
        }
    }

    pub async fn get_current_weather(
        &self,
        location: &WeatherLocation,
        units: &WeatherUnits,
    ) -> Result<WeatherData, String> {
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.as_ref() {
                if cached.fetched_at.elapsed() < self.cache_duration {
                    return Ok(cached.data.clone());
                }
            }
        }

        // Fetch fresh data
        let response = self.provider.get_current_weather(location, units).await?;

        let data = WeatherNormalizer::normalize(response);

        {
            let mut cache = self.cache.write().await;
            *cache = Some(CachedWeather {
                data: data.clone(),
                fetched_at: Instant::now(),
            });
        }

        Ok(data)
    }

    #[allow(dead_code)]
    pub async fn invalidate_cache(&self) {
        let mut cache = self.cache.write().await;
        *cache = None;
    }

    #[allow(dead_code)]
    pub fn get_provider_name(&self) -> &'static str {
        self.provider.get_name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::weather::open_meteo::OpenMeteoProvider;
    use std::time::Duration;

    #[test]
    fn test_client_creation() {
        let provider = Arc::new(OpenMeteoProvider::new());
        let client = WeatherClient::new(provider, Duration::from_secs(60));
        assert_eq!(client.get_provider_name(), "Open-Meteo");
    }

    #[tokio::test]
    async fn test_cache_invalidation() {
        let provider = Arc::new(OpenMeteoProvider::new());
        let client = WeatherClient::new(provider, Duration::from_secs(60));

        client.invalidate_cache().await;

        let cache = client.cache.read().await;
        assert!(cache.is_none());
    }
}
