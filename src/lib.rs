#![doc = include_str!("../README.md")]

mod systems;

use std::path::{Path, PathBuf};

use bevy::{
    prelude::{App, Component, Event, Plugin, Resource, Update},
    tasks::Task,
    utils::hashbrown::HashMap, reflect::Reflect,
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
        std::fs::create_dir_all(format!("assets/{tiles_directory}")).unwrap();

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

    pub fn get_tiles_directory_string(&self) -> String {
        self.tiles_directory.as_path().to_str().unwrap().to_string()
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

// Unique representation of a slippy tile download task.
#[derive(Eq, PartialEq, Hash, Clone)]
pub struct SlippyTileDownloadTaskKey {
    slippy_tile_coordinates: SlippyTileCoordinates,
    zoom_level: ZoomLevel,
    tile_size: TileSize,
}

/// HashMap that keeps track of the slippy tiles that have been downloaded.
#[derive(Resource)]
pub struct SlippyTileDownloadStatus(HashMap<SlippyTileDownloadTaskKey, TileDownloadStatus>);

impl SlippyTileDownloadStatus {
    pub fn new() -> SlippyTileDownloadStatus {
        SlippyTileDownloadStatus(HashMap::new())
    }

    pub fn insert(
        &mut self,
        x: u32,
        y: u32,
        zoom_level: ZoomLevel,
        tile_size: TileSize,
        filename: String,
        download_status: DownloadStatus,
    ) {
        self.insert_with_coords(
            SlippyTileCoordinates { x, y },
            zoom_level,
            tile_size,
            filename,
            download_status,
        );
    }

    pub fn insert_with_coords(
        &mut self,
        slippy_tile_coordinates: SlippyTileCoordinates,
        zoom_level: ZoomLevel,
        tile_size: TileSize,
        filename: String,
        download_status: DownloadStatus,
    ) {
        self.0.insert(
            SlippyTileDownloadTaskKey {
                slippy_tile_coordinates,
                zoom_level,
                tile_size,
            },
            TileDownloadStatus {
                path: Path::new(&filename).to_path_buf(),
                load_status: download_status,
            },
        );
    }

    pub fn contains_key(&self, x: u32, y: u32, zoom_level: ZoomLevel, tile_size: TileSize) -> bool {
        self.contains_key_with_coords(SlippyTileCoordinates { x, y }, zoom_level, tile_size)
    }

    pub fn contains_key_with_coords(
        &self,
        slippy_tile_coordinates: SlippyTileCoordinates,
        zoom_level: ZoomLevel,
        tile_size: TileSize,
    ) -> bool {
        self.0.contains_key(&SlippyTileDownloadTaskKey {
            slippy_tile_coordinates,
            zoom_level,
            tile_size,
        })
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
    HashMap<SlippyTileDownloadTaskKey, Task<SlippyTileDownloadTaskResult>>,
);

impl SlippyTileDownloadTasks {
    pub fn new() -> SlippyTileDownloadTasks {
        SlippyTileDownloadTasks(HashMap::new())
    }

    pub fn insert(
        &mut self,
        x: u32,
        y: u32,
        zoom_level: ZoomLevel,
        tile_size: TileSize,
        task: Task<SlippyTileDownloadTaskResult>,
    ) {
        self.insert_with_coords(SlippyTileCoordinates { x, y }, zoom_level, tile_size, task);
    }

    pub fn insert_with_coords(
        &mut self,
        slippy_tile_coordinates: SlippyTileCoordinates,
        zoom_level: ZoomLevel,
        tile_size: TileSize,
        task: Task<SlippyTileDownloadTaskResult>,
    ) {
        self.0.insert(
            SlippyTileDownloadTaskKey {
                slippy_tile_coordinates,
                zoom_level,
                tile_size,
            },
            task,
        );
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
#[derive(Debug, Event)]
pub struct DownloadSlippyTilesEvent {
    pub tile_size: TileSize,
    pub zoom_level: ZoomLevel,
    pub coordinates: Coordinates,
    /// The number of surrounding layers of tiles to request along with the center tile.
    pub radius: Radius,
    /// If set to false, will force download of new tiles from the endpoint regardless of previous requests and tiles already on disk.
    pub use_cache: bool,
}

impl DownloadSlippyTilesEvent {
    pub fn get_slippy_tile_coordinates(&self) -> SlippyTileCoordinates {
        self.coordinates
            .get_slippy_tile_coordinates(self.zoom_level)
    }
}

/// The library will generate these events upon successful slippy tile downloads.
#[derive(Debug, Event)]
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
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Component, Reflect)]
pub struct SlippyTileCoordinates {
    pub x: u32,
    pub y: u32,
}

impl SlippyTileCoordinates {
    /// Get slippy tile coordinates based on a real-world lat/lon and zoom level.
    pub fn from_lat_lon_zoom(lat: f64, lon: f64, zoom_level: ZoomLevel) -> SlippyTileCoordinates {
        let x = longitude_to_tile(lon, zoom_level.to_u8());
        let y = latitude_to_tile(lat, zoom_level.to_u8());
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
        SlippyTileCoordinates::from_lat_lon_zoom(self.latitude, self.longitude, zoom_level)
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

pub enum UseCache {
    Yes,
    No,
}

impl UseCache {
    pub fn new(value: bool) -> UseCache {
        match value {
            true => UseCache::Yes,
            _ => UseCache::No,
        }
    }
}

pub enum AlreadyDownloaded {
    Yes,
    No,
}

impl AlreadyDownloaded {
    pub fn new(value: bool) -> AlreadyDownloaded {
        match value {
            true => AlreadyDownloaded::Yes,
            _ => AlreadyDownloaded::No,
        }
    }
}

pub enum FileExists {
    Yes,
    No,
}

impl FileExists {
    pub fn new(value: bool) -> FileExists {
        match value {
            true => FileExists::Yes,
            _ => FileExists::No,
        }
    }
}

pub fn latitude_to_tile(lat: f64, zoom: u8) -> u32 {
    ((1.0 - ((lat * std::f64::consts::PI / 180.0).tan() + 1.0 / (lat * std::f64::consts::PI / 180.0).cos()).ln() / std::f64::consts::PI) / 2.0 * f64::powf(2.0,zoom as f64)).floor() as u32
}

pub fn longitude_to_tile(lon: f64, zoom: u8) -> u32 {
    ((lon + 180.0) / 360.0 * f64::powf(2.0,zoom as f64)).floor() as u32
}

pub struct SlippyTilesPlugin;

impl Plugin for SlippyTilesPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SlippyTileDownloadStatus::new())
            .insert_resource(SlippyTileDownloadTasks::new())
            .add_event::<DownloadSlippyTilesEvent>()
            .add_event::<SlippyTileDownloadedEvent>()
            .add_systems(Update, systems::download_slippy_tiles)
            .add_systems(Update, systems::download_slippy_tiles_completed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_size_new() {
        assert_eq!(TileSize::new(256), TileSize::Normal);
        assert_eq!(TileSize::new(512), TileSize::Large);
        assert_eq!(TileSize::new(1024), TileSize::Normal);
    }

    #[test]
    fn test_slippy_tile_settings() {
        let sts = SlippyTilesSettings::new("endpoint", "tiles_directory");
        assert_eq!(sts.get_endpoint(), "endpoint");
        assert_eq!(sts.get_tiles_directory(), PathBuf::from("tiles_directory"));
        assert_eq!(sts.get_tiles_directory_string(), "tiles_directory");
        assert!(std::path::Path::try_exists("assets/tiles_directory".as_ref()).is_ok());
    }

    #[test]
    fn test_slippy_tile_coordinates() {
        //assert_eq!(
            //SlippyTileCoordinates::from_lat_lon_zoom(0.0045, 0.0045, ZoomLevel::L1),
            //SlippyTileCoordinates { x: 1, y: 1 }
        //);
        //assert_eq!(
            //SlippyTileCoordinates::from_lat_lon_zoom(0.0045, 0.0045, ZoomLevel::L10),
            //SlippyTileCoordinates { x: 512, y: 512 }
        //);
        //assert_eq!(
            //SlippyTileCoordinates::from_lat_lon_zoom(0.0045, 0.0045, ZoomLevel::L19),
            //SlippyTileCoordinates {
                //x: 262151,
                //y: 262137
            //}
        //);
        assert_eq!(
            SlippyTileCoordinates::from_lat_lon_zoom(26.85, 72.58, ZoomLevel::L19),
            SlippyTileCoordinates {
                x: 367846,
                y: 221526
            }
        );
    }

    #[test]
    fn test_slippy_tile_download_status() {
        let mut stds = SlippyTileDownloadStatus::default();
        stds.insert(
            100,
            50,
            ZoomLevel::L10,
            TileSize::Normal,
            "filename".into(),
            DownloadStatus::Downloading,
        );
        assert!(!stds.contains_key(100, 50, ZoomLevel::L1, TileSize::Normal));
        assert!(!stds.contains_key(100, 50, ZoomLevel::L10, TileSize::Large));
        assert!(!stds.contains_key(100, 100, ZoomLevel::L10, TileSize::Normal));
        assert!(stds.contains_key(100, 50, ZoomLevel::L10, TileSize::Normal));
        stds.insert(
            50,
            100,
            ZoomLevel::L18,
            TileSize::Large,
            "filename".into(),
            DownloadStatus::Downloaded,
        );
        assert!(!stds.contains_key(50, 100, ZoomLevel::L1, TileSize::Large));
        assert!(!stds.contains_key(50, 100, ZoomLevel::L18, TileSize::Normal));
        assert!(!stds.contains_key(100, 50, ZoomLevel::L18, TileSize::Large));
        assert!(stds.contains_key(50, 100, ZoomLevel::L18, TileSize::Large));
    }
}
