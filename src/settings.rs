use bevy::prelude::Resource;
use std::{path::PathBuf, time::Duration};

/// Type used to dictate various settings for this crate.
///
/// `endpoint` - Tile server endpoint (example: <https://tile.openstreetmap.org>).
/// TODO: Choose a few as backup and abide by usage policy - <https://wiki.openstreetmap.org/wiki/Tile_servers>
///
/// `tiles_directory` - The folder that all tiles will be stored in.
/// 
/// `max_concurrent_downloads` - Maximum number of concurrent tile downloads.
/// 
/// `max_retries` - Maximum number of retry attempts for failed downloads.
/// 
/// `rate_limit_requests` - Maximum number of requests allowed within the rate limit window.
/// 
/// `rate_limit_window` - Duration of the rate limit window.
#[derive(Clone, Resource)]
pub struct SlippyTilesSettings {
    pub endpoint: String,
    pub tiles_directory: PathBuf,
    pub max_concurrent_downloads: usize,
    pub max_retries: u32,
    pub rate_limit_requests: usize,
    pub rate_limit_window: Duration,
}

impl SlippyTilesSettings {
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

impl Default for SlippyTilesSettings {
    fn default() -> Self {
        Self {
            endpoint: "https://tile.openstreetmap.org".into(),
            tiles_directory: PathBuf::from("tiles/"),
            max_concurrent_downloads: 4,
            max_retries: 3,
            rate_limit_requests: 10,
            rate_limit_window: Duration::from_secs(1),
        }
    }
}
