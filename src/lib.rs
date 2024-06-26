#![doc = include_str!("../README.md")]

mod systems;

use std::path::{Path, PathBuf};

use bevy::{
    prelude::{App, Component, Event, Plugin, Resource, Update},
    tasks::Task,
    utils::hashbrown::HashMap,
};

pub const EARTH_CIRCUMFERENCE: f64 = 40_075_016.686;
pub const EARTH_RADIUS: f64 = 6_378_137_f64;
pub const DEGREES_PER_METER: f64 = 360.0 / EARTH_CIRCUMFERENCE;
pub const METERS_PER_DEGREE: f64 = EARTH_CIRCUMFERENCE / 360.0;

// TODO: Incorporate many of the functions here:
//       https://hackage.haskell.org/package/tile-0.3.0.0/src/src/Data/Tile.hs

/// The number of meters per pixel at the equator.
/// https://wiki.openstreetmap.org/wiki/Slippy_map_tilenames#Resolution_and_Scale
pub fn meters_per_pixel(zoom_level: ZoomLevel) -> f64 {
    match zoom_level {
        ZoomLevel::L0 => 156543.03,
        ZoomLevel::L1 => 78271.52,
        ZoomLevel::L2 => 39135.76,
        ZoomLevel::L3 => 19567.88,
        ZoomLevel::L4 => 9783.94,
        ZoomLevel::L5 => 4891.97,
        ZoomLevel::L6 => 2445.98,
        ZoomLevel::L7 => 1222.99,
        ZoomLevel::L8 => 611.50,
        ZoomLevel::L9 => 305.75,
        ZoomLevel::L10 => 152.87,
        ZoomLevel::L11 => 76.437,
        ZoomLevel::L12 => 38.219,
        ZoomLevel::L13 => 19.109,
        ZoomLevel::L14 => 9.5546,
        ZoomLevel::L15 => 4.7773,
        ZoomLevel::L16 => 2.3887,
        ZoomLevel::L17 => 1.1943,
        ZoomLevel::L18 => 0.5972,
        ZoomLevel::L19 => 0.2986,
        ZoomLevel::L20 => 0.1493,
        ZoomLevel::L21 => 0.0747,
        ZoomLevel::L22 => 0.0374,
        ZoomLevel::L23 => 0.0187,
        ZoomLevel::L24 => 0.0094,
        ZoomLevel::L25 => 0.0047,
    }
}

/// Type used to dictate various settings for this crate.
///
/// `endpoint` - Tile server endpoint (example: <https://tile.openstreetmap.org>).
/// TODO: Choose a few as backup and abide by usage policy - <https://wiki.openstreetmap.org/wiki/Tile_servers>
///
/// `tiles_directory` - The folder that all tiles will be stored in.
#[derive(Clone, Resource)]
pub struct SlippyTilesSettings {
    endpoint: String,
    tiles_directory: PathBuf,
}

impl SlippyTilesSettings {
    /// Used to initialize slippy tile settings without using default values.
    /// Will also create the tiles directory immediately.
    pub fn new(endpoint: &str, tiles_directory: &str) -> SlippyTilesSettings {
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
    L20,
    L21,
    L22,
    L23,
    L24,
    L25,
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
            ZoomLevel::L20 => 20,
            ZoomLevel::L21 => 21,
            ZoomLevel::L22 => 22,
            ZoomLevel::L23 => 23,
            ZoomLevel::L24 => 24,
            ZoomLevel::L25 => 25,
        }
    }
}

/// The size of the tiles being requested - either 256px (Normal), 512px (Large), or 768px (VeryLarge).
/// Not every tile provider supports Large and VeryLarge.
#[derive(Eq, Hash, PartialEq, Clone, Copy, Debug)]
pub enum TileSize {
    Normal,
    Large,
    VeryLarge,
}

impl TileSize {
    /// Create a new TileSize type given a pixel count (512px = TileSize::Large, every other value is TileSize::Normal).
    pub fn new(tile_pixels: u32) -> TileSize {
        match tile_pixels {
            768 => TileSize::VeryLarge,
            512 => TileSize::Large,
            _ => TileSize::Normal,
        }
    }

    /// Returns the number of tile pixels given a TileSize variant.
    pub fn to_pixels(&self) -> u32 {
        match self {
            TileSize::Normal => 256,
            TileSize::Large => 512,
            TileSize::VeryLarge => 768,
        }
    }

    pub fn get_url_postfix(&self) -> String {
        match self {
            TileSize::Normal => "".into(),
            TileSize::Large => "@2x".into(),
            TileSize::VeryLarge => "@3x".into(),
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
        assert_eq!(
            SlippyTileCoordinates::from_latitude_longitude(85.0511287798066, 0.0, ZoomLevel::L1),
            SlippyTileCoordinates { x: 1, y: 0 }
        );
        assert_eq!(
            SlippyTileCoordinates::to_latitude_longitude(
                &SlippyTileCoordinates { x: 1, y: 0 },
                ZoomLevel::L1
            ),
            LatitudeLongitudeCoordinates {
                latitude: 85.0511287798066,
                longitude: 0.0
            }
        );
        assert_eq!(
            SlippyTileCoordinates::from_latitude_longitude(0.0, 0.0, ZoomLevel::L10),
            SlippyTileCoordinates { x: 512, y: 512 }
        );
        assert_eq!(
            SlippyTileCoordinates::to_latitude_longitude(
                &SlippyTileCoordinates { x: 512, y: 512 },
                ZoomLevel::L10
            ),
            LatitudeLongitudeCoordinates {
                latitude: 0.0,
                longitude: 0.0
            }
        );
        assert_eq!(
            SlippyTileCoordinates::from_latitude_longitude(
                48.81590713080016,
                2.2686767578125,
                ZoomLevel::L17
            ),
            SlippyTileCoordinates { x: 66362, y: 45115 }
        );
        assert_eq!(
            SlippyTileCoordinates::to_latitude_longitude(
                &SlippyTileCoordinates { x: 66362, y: 45115 },
                ZoomLevel::L17
            ),
            LatitudeLongitudeCoordinates {
                latitude: 48.81590713080016,
                longitude: 2.2686767578125
            }
        );
        assert_eq!(
            SlippyTileCoordinates::from_latitude_longitude(
                0.004806518549043551,
                0.004119873046875,
                ZoomLevel::L19
            ),
            SlippyTileCoordinates {
                x: 262150,
                y: 262137
            }
        );
        assert_eq!(
            SlippyTileCoordinates::to_latitude_longitude(
                &SlippyTileCoordinates {
                    x: 262150,
                    y: 262137
                },
                ZoomLevel::L19
            ),
            LatitudeLongitudeCoordinates {
                latitude: 0.004806518549043551,
                longitude: 0.004119873046875
            }
        );
        assert_eq!(
            SlippyTileCoordinates::from_latitude_longitude(
                26.850416392948524,
                72.57980346679688,
                ZoomLevel::L19
            ),
            SlippyTileCoordinates {
                x: 367846,
                y: 221525
            }
        );
        assert_eq!(
            SlippyTileCoordinates::to_latitude_longitude(
                &SlippyTileCoordinates {
                    x: 367846,
                    y: 221525
                },
                ZoomLevel::L19
            ),
            LatitudeLongitudeCoordinates {
                latitude: 26.850416392948524,
                longitude: 72.57980346679688
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

    #[test]
    fn test_pixel_to_world_coords() {
        let tile_size = TileSize::Normal;
        let zoom_level = ZoomLevel::L18;
        // Bevy start (0,0) at the bottom left, so this is close to the bottom left on a map rendered in bevy.
        let world_coords = world_pixel_to_world_coords(42_125.0, 101_661.0, tile_size, zoom_level);
        let rounded = LatitudeLongitudeCoordinates {
            latitude: format!("{:.5}", world_coords.latitude).parse().unwrap(),
            longitude: format!("{:.5}", world_coords.longitude).parse().unwrap(),
        };
        // We expect world coordinates roughly in the bottom left on a world map.
        let check = LatitudeLongitudeCoordinates {
            latitude: -85.00386,
            longitude: -179.77402,
        };
        assert_eq!(rounded, check);
        let pixel = world_coords_to_world_pixel(&world_coords, tile_size, zoom_level);
        let rounded = (pixel.0.round(), pixel.1.round());
        assert_eq!(rounded, (42_125.0, 101_661.0));
    }

    #[test]
    fn test_world_coords_to_pixel() {
        let world_coords = LatitudeLongitudeCoordinates {
            latitude: 45.41098,
            longitude: -75.69854,
        };
        let tile_size = TileSize::Normal;
        let zoom_level = ZoomLevel::L18;
        let pixel = world_coords_to_world_pixel(&world_coords, tile_size, zoom_level);
        assert_eq!((pixel.0 as u32, pixel.1 as u32), (19_443_201, 43_076_862));
        let world_coords2 = world_pixel_to_world_coords(pixel.0, pixel.1, tile_size, zoom_level);
        let rounded = LatitudeLongitudeCoordinates {
            latitude: format!("{:.5}", world_coords2.latitude).parse().unwrap(),
            longitude: format!("{:.5}", world_coords2.longitude).parse().unwrap(),
        };
        assert_eq!(world_coords, rounded);
    }
}
