use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct IpInfoResponse {
    loc: String,
    city: Option<String>,
}

#[derive(Debug)]
pub struct GeoLocation {
    pub latitude: f64,
    pub longitude: f64,
    pub city: Option<String>,
}

pub async fn detect_location() -> Result<GeoLocation, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get("https://ipinfo.io/json")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch location: {}", e))?;

    let ip_info: IpInfoResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse location response: {}", e))?;

    let coords: Vec<&str> = ip_info.loc.split(',').collect();
    if coords.len() != 2 {
        return Err("Invalid location format from ipinfo.io".to_string());
    }

    let latitude = coords[0]
        .parse::<f64>()
        .map_err(|e| format!("Invalid latitude: {}", e))?;
    let longitude = coords[1]
        .parse::<f64>()
        .map_err(|e| format!("Invalid longitude: {}", e))?;

    Ok(GeoLocation {
        latitude,
        longitude,
        city: ip_info.city,
    })
}
