use std::path::{Path, PathBuf};

use bevy::{
    ecs::message::Message, prelude::Resource, tasks::Task
};
use bevy_platform::collections::HashMap;

use crate::coordinates::{Coordinates, SlippyTileCoordinates};
use crate::types::{DownloadStatus, TileSize, ZoomLevel};

// Unique representation of a slippy tile download task.
#[derive(Eq, PartialEq, Hash, Clone)]
pub struct SlippyTileDownloadTaskKey {
    pub slippy_tile_coordinates: SlippyTileCoordinates,
    pub zoom_level: ZoomLevel,
    pub tile_size: TileSize,
}

/// HashMap that keeps track of the slippy tiles that have been downloaded.
#[derive(Resource)]
pub struct SlippyTileDownloadStatus(pub HashMap<SlippyTileDownloadTaskKey, TileDownloadStatus>);

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
    pub HashMap<SlippyTileDownloadTaskKey, Task<SlippyTileDownloadTaskResult>>,
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

/// Users send these events to request slippy tile downloads.
#[derive(Debug, Message)]
pub struct DownloadSlippyTilesEvent {
    pub tile_size: TileSize,
    pub zoom_level: ZoomLevel,
    pub coordinates: Coordinates,
    /// The number of surrounding layers of tiles to request along with the center tile.
    pub radius: crate::types::Radius,
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
#[derive(Debug, Message)]
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
