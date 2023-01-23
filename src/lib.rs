#![doc = include_str!("../README.md")]

mod systems;

use std::path::PathBuf;

use bevy::{
    prelude::{App, Component, Plugin, Resource},
    tasks::Task,
    utils::hashbrown::HashMap,
};

/// Type used to dictate various settings for this crate.
///
/// `endpoint` - Tile server endpoint (example: <https://tile.openstreetmap.org>).
/// TODO: Choose a few as backup and abide by usage policy - <https://wiki.openstreetmap.org/wiki/Tile_servers>
///
/// `tiles_directory` - The folder that all tiles will be stored in.
///
/// `tile_size` - [TileSize::Normal] (256px) or [TileSize::Large] (512px). Large is not supported for many tile servers.
///
/// `zoom_level` - The slippy tile zoom level. Can be between [ZoomLevel::L0] to [ZoomLevel::L19].
#[derive(Clone, Resource)]
pub struct SlippyTilesSettings {
    endpoint: String,
    tiles_directory: PathBuf,
}

impl SlippyTilesSettings {
    /// Used to initialize slippy tile settings without using default values.
    /// Will also create the tiles directory immediately.
    pub fn new(endpoint: &str, tiles_directory: &str) -> SlippyTilesSettings {
        // Need to ensure tiles folder exists.
        std::fs::create_dir_all(format!("assets/{}", tiles_directory)).unwrap();

        SlippyTilesSettings {
            endpoint: endpoint.to_owned(),
            tiles_directory: PathBuf::from(tiles_directory),
        }
    }

    pub fn get_endpoint(&self) -> String {
        self.endpoint.clone()
    }

    pub fn get_tiles_directory(&self) -> PathBuf {
        self.tiles_directory.clone()
    }
}

/// Default values include localhost:8080 for the endpoint, `tiles/` for the directory, TileSize::Normal, and ZoomLevel(18).
impl Default for SlippyTilesSettings {
    fn default() -> Self {
        Self::new("http://localhost:8080", "tiles/")
    }
}

/// The zoom level used when fetching tiles (0 <= zoom <= 19)
#[derive(Eq, Hash, PartialEq, Clone, Copy, Debug)]
pub enum ZoomLevel {
    L0,
    L1,
    L2,
    L3,
    L4,
    L5,
    L6,
    L7,
    L8,
    L9,
    L10,
    L11,
    L12,
    L13,
    L14,
    L15,
    L16,
    L17,
    L18,
    L19,
}

impl ZoomLevel {
    pub fn to_u8(&self) -> u8 {
        match self {
            ZoomLevel::L0 => 0,
            ZoomLevel::L1 => 1,
            ZoomLevel::L2 => 2,
            ZoomLevel::L3 => 3,
            ZoomLevel::L4 => 4,
            ZoomLevel::L5 => 5,
            ZoomLevel::L6 => 6,
            ZoomLevel::L7 => 7,
            ZoomLevel::L8 => 8,
            ZoomLevel::L9 => 9,
            ZoomLevel::L10 => 10,
            ZoomLevel::L11 => 11,
            ZoomLevel::L12 => 12,
            ZoomLevel::L13 => 13,
            ZoomLevel::L14 => 14,
            ZoomLevel::L15 => 15,
            ZoomLevel::L16 => 16,
            ZoomLevel::L17 => 17,
            ZoomLevel::L18 => 18,
            ZoomLevel::L19 => 19,
        }
    }
}

/// The size of the tiles being requested - either 256px (Normal), or 512px (Large).
/// Not every tile provider supports Large.
#[derive(Eq, Hash, PartialEq, Clone, Copy, Debug)]
pub enum TileSize {
    Normal,
    Large,
}

impl TileSize {
    /// Create a new TileSize type given a pixel count (512px = TileSize::Large, every other value is TileSize::Normal).
    pub fn new(tile_pixels: u32) -> TileSize {
        match tile_pixels {
            512 => TileSize::Large,
            _ => TileSize::Normal,
        }
    }

    /// Returns the number of tile pixels given a TileSize variant.
    pub fn to_pixels(&self) -> u32 {
        match self {
            TileSize::Normal => 256,
            TileSize::Large => 512,
        }
    }
}

/// Number of tiles away from the main tile that should be fetched. Effectively translates to layers of surrounding tiles. Will degrade performance exponentially.
///
/// Radius(0) = 1 tile (1x1), Radius(1) = 9 tiles (3x3), Radius(2) = 25 tiles (5x5), Radius(3) = 49 tiles (7x7), etc.
#[derive(Debug)]
pub struct Radius(pub u8);

/// HashMap that keeps track of the slippy tiles that have been downloaded.
#[derive(Resource)]
pub struct SlippyTileDownloadStatus(
    HashMap<(SlippyTileCoordinates, ZoomLevel, TileSize), TileDownloadStatus>,
);

impl SlippyTileDownloadStatus {
    pub fn new() -> SlippyTileDownloadStatus {
        SlippyTileDownloadStatus(HashMap::new())
    }
}

impl Default for SlippyTileDownloadStatus {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents the download status of a single slippy tile.
pub struct TileDownloadStatus {
    pub path: PathBuf,
    pub load_status: DownloadStatus,
}

/// A wrapper type that represents the results of the async task used to download tiles.
/// Contains the path of the tile downloaded.
#[derive(Clone)]
pub struct SlippyTileDownloadTaskResult {
    pub path: PathBuf,
}

/// HashMap of all tiles currently being downloaded.
#[derive(Resource)]
pub struct SlippyTileDownloadTasks(
    HashMap<(SlippyTileCoordinates, ZoomLevel, TileSize), Task<SlippyTileDownloadTaskResult>>,
);

impl SlippyTileDownloadTasks {
    pub fn new() -> SlippyTileDownloadTasks {
        SlippyTileDownloadTasks(HashMap::new())
    }
}

impl Default for SlippyTileDownloadTasks {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents the status of tiles being downloaded.
pub enum DownloadStatus {
    Downloading,
    Downloaded,
}

/// Users send these events to request slippy tile downloads.
#[derive(Debug)]
pub struct DownloadSlippyTilesEvent {
    pub tile_size: TileSize,
    pub zoom_level: ZoomLevel,
    pub coordinates: Coordinates,
    /// The number of surrounding layers of tiles to request along with the center tile.
    pub radius: Radius,
    /// If set to false, will force download of new tiles from the endpoint regardless of previous requests and tiles already on disk.
    pub use_cache: bool,
}

/// The library will generate these events upon successful slippy tile downloads.
#[derive(Debug)]
pub struct SlippyTileDownloadedEvent {
    /// The [`TileSize`] used for this downloaded slippy tile.
    pub tile_size: TileSize,
    /// The [`ZoomLevel`] used for this downloaded slippy tile.
    pub zoom_level: ZoomLevel,
    /// The [`Coordinates`] used for this downloaded slippy tile.
    pub coordinates: Coordinates,
    /// The assets/ path where the slippy tile was downloaded - can be used directly with the [`AssetServer`].
    pub path: PathBuf,
}

impl SlippyTileDownloadedEvent {
    pub fn get_slippy_tile_coordinates(&self) -> SlippyTileCoordinates {
        self.coordinates
            .get_slippy_tile_coordinates(self.zoom_level)
    }
}

/// Slippy map tile coordinates: <https://wiki.openstreetmap.org/wiki/Slippy_map_tilenames>
/// The x and y coordinates are used directly in the endpoint download requests.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Component)]
pub struct SlippyTileCoordinates {
    pub x: u32,
    pub y: u32,
}

impl SlippyTileCoordinates {
    /// Get slippy tile coordinates based on a real-world lat/lon and zoom level.
    pub fn from_lat_lon_zoom(lat: f64, lon: f64, zoom_level: ZoomLevel) -> SlippyTileCoordinates {
        let lat_rad = lat.to_radians();
        let num_tiles = f64::powf(2.0, zoom_level.to_u8() as f64);
        let x = ((lon + 180.0_f64) / 360.0_f64 * num_tiles).round() as u32;
        let y = ((1.0_f64 - (lat_rad.tan()).asinh() / std::f64::consts::PI) / 2.0_f64 * num_tiles)
            .round() as u32;
        SlippyTileCoordinates { x, y }
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
        let lat_rad = self.latitude.to_radians();
        let num_tiles = f64::powf(2.0, zoom_level.to_u8() as f64);
        let x = ((self.longitude + 180.0_f64) / 360.0_f64 * num_tiles).round() as u32;
        let y = ((1.0_f64 - (lat_rad.tan()).asinh() / std::f64::consts::PI) / 2.0_f64 * num_tiles)
            .round() as u32;
        SlippyTileCoordinates { x, y }
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

pub struct SlippyTilesPlugin;

impl Plugin for SlippyTilesPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SlippyTileDownloadStatus::new())
            .insert_resource(SlippyTileDownloadTasks::new())
            .add_event::<DownloadSlippyTilesEvent>()
            .add_event::<SlippyTileDownloadedEvent>()
            .add_system(systems::download_slippy_tiles)
            .add_system(systems::download_slippy_tiles_completed);
    }
}
