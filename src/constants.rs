pub const EARTH_CIRCUMFERENCE: f64 = 40_075_016.686;
pub const EARTH_RADIUS: f64 = 6_378_137_f64;
pub const DEGREES_PER_METER: f64 = 360.0 / EARTH_CIRCUMFERENCE;
pub const METERS_PER_DEGREE: f64 = EARTH_CIRCUMFERENCE / 360.0;

/// https://wiki.openstreetmap.org/wiki/Slippy_map_tilenames#Resolution_and_Scale
pub fn meters_per_pixel(zoom_level: crate::types::ZoomLevel, latitude: f64, tile_size: crate::types::TileSize) -> f64 {
    let base_resolution = EARTH_CIRCUMFERENCE / tile_size.to_pixels() as f64;
    let latitude_radians = latitude.to_radians();
    base_resolution * latitude_radians.cos() / (1_u32 << zoom_level.to_u8()) as f64
}