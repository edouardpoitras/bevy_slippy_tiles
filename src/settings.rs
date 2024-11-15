use bevy::prelude::Resource;
use std::path::PathBuf;

/// Type used to dictate various settings for this crate.
///
/// `endpoint` - Tile server endpoint (example: <https://tile.openstreetmap.org>).
/// TODO: Choose a few as backup and abide by usage policy - <https://wiki.openstreetmap.org/wiki/Tile_servers>
///
/// `tiles_directory` - The folder that all tiles will be stored in.
#[derive(Clone, Resource)]
pub struct SlippyTilesSettings {
    pub endpoint: String,
    pub tiles_directory: PathBuf,
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
