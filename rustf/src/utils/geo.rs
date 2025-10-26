//! Geographic utilities for RustF framework
//!
//! This module provides functions for geographic calculations commonly used
//! in location-based web applications.

/// Calculate distance between two geographic points in kilometers using Haversine formula
///
/// # Arguments
/// * `lat1` - Latitude of first point in degrees
/// * `lon1` - Longitude of first point in degrees
/// * `lat2` - Latitude of second point in degrees
/// * `lon2` - Longitude of second point in degrees
///
/// # Example
/// ```rust,ignore
/// // Distance between New York and Los Angeles
/// let distance = distance(40.7128, -74.0060, 34.0522, -118.2437);
/// println!("Distance: {:.2} km", distance); // ~3944 km
/// ```
pub fn distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const EARTH_RADIUS_KM: f64 = 6371.0;

    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();
    let delta_lat = (lat2 - lat1).to_radians();
    let delta_lon = (lon2 - lon1).to_radians();

    let a = (delta_lat / 2.0).sin().powi(2)
        + lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);

    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    EARTH_RADIUS_KM * c
}

/// Calculate distance in miles
///
/// # Arguments
/// * `lat1` - Latitude of first point in degrees
/// * `lon1` - Longitude of first point in degrees
/// * `lat2` - Latitude of second point in degrees
/// * `lon2` - Longitude of second point in degrees
///
/// # Example
/// ```rust,ignore
/// let distance_miles = distance_miles(40.7128, -74.0060, 34.0522, -118.2437);
/// println!("Distance: {:.2} miles", distance_miles);
/// ```
pub fn distance_miles(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    distance(lat1, lon1, lat2, lon2) * 0.621371
}

/// Check if a point is within a bounding box
///
/// # Arguments
/// * `lat` - Latitude of point to check
/// * `lon` - Longitude of point to check
/// * `min_lat` - Minimum latitude of bounding box
/// * `max_lat` - Maximum latitude of bounding box
/// * `min_lon` - Minimum longitude of bounding box
/// * `max_lon` - Maximum longitude of bounding box
///
/// # Example
/// ```rust,ignore
/// let in_bounds = in_bounds(40.7128, -74.0060, 40.0, 41.0, -75.0, -73.0);
/// assert!(in_bounds);
/// ```
pub fn in_bounds(
    lat: f64,
    lon: f64,
    min_lat: f64,
    max_lat: f64,
    min_lon: f64,
    max_lon: f64,
) -> bool {
    lat >= min_lat && lat <= max_lat && lon >= min_lon && lon <= max_lon
}

/// Calculate the bearing (direction) between two points
///
/// Returns the bearing in degrees (0-360), where 0 is North, 90 is East, etc.
///
/// # Arguments
/// * `lat1` - Latitude of first point in degrees
/// * `lon1` - Longitude of first point in degrees
/// * `lat2` - Latitude of second point in degrees
/// * `lon2` - Longitude of second point in degrees
///
/// # Example
/// ```rust,ignore
/// let bearing = bearing(40.7128, -74.0060, 34.0522, -118.2437);
/// println!("Bearing: {:.1}°", bearing);
/// ```
pub fn bearing(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();
    let delta_lon = (lon2 - lon1).to_radians();

    let y = delta_lon.sin() * lat2_rad.cos();
    let x = lat1_rad.cos() * lat2_rad.sin() - lat1_rad.sin() * lat2_rad.cos() * delta_lon.cos();

    let bearing_rad = y.atan2(x);
    (bearing_rad.to_degrees() + 360.0) % 360.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distance() {
        // Distance between New York and Los Angeles (approximately 3944 km)
        let dist = distance(40.7128, -74.0060, 34.0522, -118.2437);
        assert!(
            (dist - 3944.0).abs() < 50.0,
            "Distance should be approximately 3944 km, got {}",
            dist
        );

        // Distance between same point should be 0
        let same_point = distance(40.7128, -74.0060, 40.7128, -74.0060);
        assert!(
            same_point < 0.001,
            "Distance between same points should be ~0, got {}",
            same_point
        );
    }

    #[test]
    fn test_distance_miles() {
        let dist_km = distance(40.7128, -74.0060, 34.0522, -118.2437);
        let dist_miles = distance_miles(40.7128, -74.0060, 34.0522, -118.2437);

        // Check conversion factor
        let expected_miles = dist_km * 0.621371;
        assert!((dist_miles - expected_miles).abs() < 0.1);
    }

    #[test]
    fn test_in_bounds() {
        // Point inside bounds
        assert!(in_bounds(40.7128, -74.0060, 40.0, 41.0, -75.0, -73.0));

        // Point outside bounds (latitude)
        assert!(!in_bounds(42.0, -74.0060, 40.0, 41.0, -75.0, -73.0));

        // Point outside bounds (longitude)
        assert!(!in_bounds(40.7128, -72.0, 40.0, 41.0, -75.0, -73.0));

        // Point on boundary
        assert!(in_bounds(40.0, -74.0, 40.0, 41.0, -75.0, -73.0));
    }

    #[test]
    fn test_bearing() {
        // Bearing from New York to Los Angeles should be roughly west-southwest
        let b = bearing(40.7128, -74.0060, 34.0522, -118.2437);
        assert!(
            b > 200.0 && b < 280.0,
            "Bearing should be west-southwest, got {}",
            b
        );

        // Bearing should be 0-360 degrees
        assert!(b >= 0.0 && b < 360.0);
    }
}
