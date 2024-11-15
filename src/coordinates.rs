use crate::types::{TileSize, ZoomLevel};
use bevy::prelude::Component;

/// Slippy map tile coordinates: <https://wiki.openstreetmap.org/wiki/Slippy_map_tilenames>
/// The x and y coordinates are used directly in the endpoint download requests.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Component)]
pub struct SlippyTileCoordinates {
    pub x: u32,
    pub y: u32,
}

impl SlippyTileCoordinates {
    /// Get slippy tile coordinates based on a real-world lat/lon and zoom level.
    pub fn from_latitude_longitude(
        lat: f64,
        lon: f64,
        zoom_level: ZoomLevel,
    ) -> SlippyTileCoordinates {
        let x = longitude_to_tile_x(lon, zoom_level.to_u8());
        let y = latitude_to_tile_y(lat, zoom_level.to_u8());
        SlippyTileCoordinates { x, y }
    }

    /// Get real-world lat/lon based on slippy tile coordinates.
    pub fn to_latitude_longitude(&self, zoom_level: ZoomLevel) -> LatitudeLongitudeCoordinates {
        let lon = tile_x_to_longitude(self.x, zoom_level.to_u8());
        let lat = tile_y_to_latitude(self.y, zoom_level.to_u8());
        LatitudeLongitudeCoordinates {
            latitude: lat,
            longitude: lon,
        }
    }
}

/// Real-world latitude/longitude coordinates.
/// This format is for the user's convenicence - values get converted to SlippyTileCoordinates for the request.
#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct LatitudeLongitudeCoordinates {
    pub latitude: f64,
    pub longitude: f64,
}

impl LatitudeLongitudeCoordinates {
    /// Get slippy tile coordinates based on a real-world lat/lon and zoom level.
    pub fn to_slippy_tile_coordinates(&self, zoom_level: ZoomLevel) -> SlippyTileCoordinates {
        SlippyTileCoordinates::from_latitude_longitude(self.latitude, self.longitude, zoom_level)
    }
}

#[derive(Debug, Clone, PartialEq, Component)]
pub enum Coordinates {
    SlippyTile(SlippyTileCoordinates),
    LatitudeLongitude(LatitudeLongitudeCoordinates),
}

impl Coordinates {
    pub fn from_slippy_tile_coordinates(x: u32, y: u32) -> Coordinates {
        Coordinates::SlippyTile(SlippyTileCoordinates { x, y })
    }

    pub fn from_latitude_longitude(latitude: f64, longitude: f64) -> Coordinates {
        Coordinates::LatitudeLongitude(LatitudeLongitudeCoordinates {
            latitude,
            longitude,
        })
    }

    pub fn get_slippy_tile_coordinates(&self, zoom_level: ZoomLevel) -> SlippyTileCoordinates {
        match &self {
            Coordinates::LatitudeLongitude(coords) => coords.to_slippy_tile_coordinates(zoom_level),
            Coordinates::SlippyTile(coords) => *coords,
        }
    }
}

// https://wiki.openstreetmap.org/wiki/Slippy_map_tilenames#Implementations
pub fn latitude_to_tile_y(lat: f64, zoom: u8) -> u32 {
    ((1.0
        - ((lat * std::f64::consts::PI / 180.0).tan()
            + 1.0 / (lat * std::f64::consts::PI / 180.0).cos())
        .ln()
            / std::f64::consts::PI)
        / 2.0
        * f64::powf(2.0, zoom as f64))
    .floor() as u32
}

// https://wiki.openstreetmap.org/wiki/Slippy_map_tilenames#Implementations
pub fn longitude_to_tile_x(lon: f64, zoom: u8) -> u32 {
    ((lon + 180.0) / 360.0 * f64::powf(2.0, zoom as f64)).floor() as u32
}

// https://wiki.openstreetmap.org/wiki/Slippy_map_tilenames#Implementations
pub fn tile_y_to_latitude(y: u32, zoom: u8) -> f64 {
    let n =
        std::f64::consts::PI - 2.0 * std::f64::consts::PI * y as f64 / f64::powf(2.0, zoom as f64);
    let intermediate: f64 = 0.5 * (n.exp() - (-n).exp());
    180.0 / std::f64::consts::PI * intermediate.atan()
}

// https://wiki.openstreetmap.org/wiki/Slippy_map_tilenames#Implementations
pub fn tile_x_to_longitude(x: u32, z: u8) -> f64 {
    x as f64 / f64::powf(2.0, z as f64) * 360.0 - 180.0
}

// Get the numbers of tiles in a given dimension, x or y, at the specified map zoom level.
pub fn max_tiles_in_dimension(zoom_level: ZoomLevel) -> f64 {
    (1 << zoom_level.to_u8()) as f64
}

// Get the number of pixels in a given dimension, x or y.
pub fn max_pixels_in_dimension(zoom_level: ZoomLevel, tile_size: TileSize) -> f64 {
    tile_size.to_pixels() as f64 * max_tiles_in_dimension(zoom_level)
}

// Given a x and y pixel position in the world (0,0 at the bottom left), return the world coordinates.
pub fn world_pixel_to_world_coords(
    x_pixel: f64,
    y_pixel: f64,
    tile_size: TileSize,
    zoom_level: ZoomLevel,
) -> LatitudeLongitudeCoordinates {
    // Flip Y axis because Bevy has (0,0) at the bottom left, but the calculation is for (0,0) at the top left.
    // TODO: Cache this max pixels value?
    let max_pixels = max_pixels_in_dimension(zoom_level, tile_size);
    let y_pixel = max_pixels - y_pixel;
    let (longitude, latitude) =
        googleprojection::Mercator::with_size(tile_size.to_pixels() as usize)
            .from_pixel_to_ll(&(x_pixel, y_pixel), zoom_level.to_u8().into())
            .unwrap_or_default();
    LatitudeLongitudeCoordinates {
        latitude,
        longitude,
    }
}

// Given world coordinates, return the x and y pixel position in the world.
pub fn world_coords_to_world_pixel(
    coords: &LatitudeLongitudeCoordinates,
    tile_size: TileSize,
    zoom_level: ZoomLevel,
) -> (f64, f64) {
    let (x, y) = googleprojection::Mercator::with_size(tile_size.to_pixels() as usize)
        .from_ll_to_subpixel(
            &(coords.longitude, coords.latitude),
            zoom_level.to_u8().into(),
        )
        .unwrap_or_default();
    // Flip Y axis because Bevy has (0,0) at the bottom left, but the calculation is for (0,0) at the top left.
    // TODO: Cache this max pixels value?
    let max_pixels = max_pixels_in_dimension(zoom_level, tile_size);
    let y = max_pixels - y;
    (x, y)
}
