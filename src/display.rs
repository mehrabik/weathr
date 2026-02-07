// weathr - Terminal-based ASCII weather application
// Copyright (C) 2026 Dony Mulya

use crossterm::{
    cursor, execute,
    terminal::{Clear, ClearType},
};
use std::io;

pub struct AsciiDisplay;

impl AsciiDisplay {
    #[allow(dead_code)]
    pub fn clear_screen() -> io::Result<()> {
        execute!(io::stdout(), Clear(ClearType::All), cursor::MoveTo(0, 0))
    }

    #[allow(dead_code)]
    pub fn format_weather_info(latitude: f64, longitude: f64) -> String {
        format!(
            "Weather for: {:.2}°N, {:.2}°E | Press 'q' to quit",
            latitude, longitude
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::house::House;

    #[test]
    fn test_format_weather_info_positive_coordinates() {
        let info = AsciiDisplay::format_weather_info(52.52, 13.41);
        assert!(info.contains("52.52°N"));
        assert!(info.contains("13.41°E"));
        assert!(info.contains("Press 'q' to quit"));
    }

    #[test]
    fn test_format_weather_info_negative_coordinates() {
        let info = AsciiDisplay::format_weather_info(-33.87, -74.01);
        assert!(info.contains("-33.87°N"));
        assert!(info.contains("-74.01°E"));
    }

    #[test]
    fn test_format_weather_info_zero_coordinates() {
        let info = AsciiDisplay::format_weather_info(0.0, 0.0);
        assert!(info.contains("0.00°N"));
        assert!(info.contains("0.00°E"));
    }

    #[test]
    fn test_format_weather_info_precision() {
        let info = AsciiDisplay::format_weather_info(52.5234567, 13.4134567);
        assert!(info.contains("52.52°N"));
        assert!(info.contains("13.41°E"));
    }

    #[test]
    fn test_format_weather_info_boundary_values() {
        let info = AsciiDisplay::format_weather_info(90.0, 180.0);
        assert!(info.contains("90.00°N"));
        assert!(info.contains("180.00°E"));
    }

    #[test]
    fn test_house_structure() {
        let house = House::default();
        let ascii = house.get_ascii();
        assert!(!ascii.is_empty());
        let house_str = ascii.join("\n");
        assert!(house_str.contains("___"));
        assert!(house_str.contains("|"));
    }
}
