use bevy::{
    ecs::event::EventReader,
    prelude::{debug, EventWriter, Res, ResMut},
    tasks::IoTaskPool,
};
use futures_lite::future;
use std::path::Path;

use crate::{
    Coordinates, DownloadSlippyTilesEvent, DownloadStatus, SlippyTileCoordinates,
    SlippyTileDownloadStatus, SlippyTileDownloadTaskResult, SlippyTileDownloadTasks,
    SlippyTileDownloadedEvent, SlippyTilesSettings, TileDownloadStatus, TileSize, ZoomLevel,
};

// System that listens for DownloadSlippyTiles events and fires off a thread to do the work.
pub fn download_slippy_tiles(
    mut download_slippy_tile_events: EventReader<DownloadSlippyTilesEvent>,
    slippy_tiles_settings: Res<SlippyTilesSettings>,
    mut slippy_tile_download_status: ResMut<SlippyTileDownloadStatus>,
    mut slippy_tile_download_tasks: ResMut<SlippyTileDownloadTasks>,
) {
    let thread_pool = IoTaskPool::get();
    for download_slippy_tile in download_slippy_tile_events.iter() {
        let zoom_level = download_slippy_tile.zoom_level;
        let tile_size = download_slippy_tile.tile_size;
        let radius = download_slippy_tile.radius.0;
        let use_cache = download_slippy_tile.use_cache;
        let slippy_tile_coordinates = download_slippy_tile
            .coordinates
            .get_slippy_tile_coordinates(zoom_level);
        for x in
            slippy_tile_coordinates.x - radius as u32..slippy_tile_coordinates.x + radius as u32 + 1
        {
            for y in slippy_tile_coordinates.y - radius as u32
                ..slippy_tile_coordinates.y + radius as u32 + 1
            {
                let spc = SlippyTileCoordinates { x, y };
                let filename_string = slippy_tiles_settings
                    .tiles_directory
                    .as_path()
                    .to_str()
                    .unwrap();
                let filename = format!(
                    "{}{}.{}.{}.tile.png",
                    filename_string,
                    zoom_level.to_u8(),
                    spc.x,
                    spc.y
                );
                let already_downloaded = slippy_tile_download_status
                    .0
                    .contains_key(&(spc, zoom_level, tile_size));
                // Save us the disk read if we've already fetched (unless cache is disabled).
                if !already_downloaded || !use_cache {
                    let file_exists =
                        std::path::Path::new(&format!("assets/{}", filename)).exists();
                    if !file_exists || !use_cache {
                        debug!(
                            "Fetching map tile at position {:?} with zoom level {:?}",
                            spc, zoom_level
                        );
                        let endpoint = slippy_tiles_settings.endpoint.clone();
                        let thread = thread_pool.spawn(async move {
                            let target = match tile_size {
                                TileSize::Normal => {
                                    format!("{}/{}/{}/{}.png", endpoint, zoom_level.to_u8(), x, y)
                                },
                                TileSize::Large => {
                                    format!("{}/{}/{}/{}@2.png", endpoint, zoom_level.to_u8(), x, y)
                                },
                            };
                            let client = reqwest::blocking::Client::new();
                            let response = client
                                .get(&target)
                                .header(reqwest::header::USER_AGENT, "bevy_slippy_tiles")
                                .send()
                                .expect("Failed to fetch tile image");
                            let mut content = std::io::Cursor::new(
                                response
                                    .bytes()
                                    .expect("Could not get tile image from bytes"),
                            );
                            let mut file_out =
                                std::fs::File::create(format!("assets/{}", filename)).unwrap();
                            std::io::copy(&mut content, &mut file_out).unwrap();
                            SlippyTileDownloadTaskResult {
                                path: Path::new(&filename).to_path_buf(),
                            }
                        });

                        // Store our active tile download tasks.
                        slippy_tile_download_tasks.0.insert(
                            (SlippyTileCoordinates { x, y }, zoom_level, tile_size),
                            thread,
                        );

                        // Cache our downloaded tiles.
                        // Have to re-define filename here because it was already moved into the thread above.
                        let filename = format!(
                            "{}{}.{}.{}.tile.png",
                            filename_string,
                            zoom_level.to_u8(),
                            spc.x,
                            spc.y
                        );
                        slippy_tile_download_status.0.insert(
                            (spc, zoom_level, tile_size),
                            TileDownloadStatus {
                                path: Path::new(&filename).to_path_buf(),
                                load_status: DownloadStatus::Downloading,
                            },
                        );
                    } else {
                        debug!("Loading slippy tile {:?} - from {}", spc, filename);
                        let thread = thread_pool.spawn(async move {
                            SlippyTileDownloadTaskResult {
                                path: Path::new(&filename).to_path_buf(),
                            }
                        });

                        // Store our active tile download tasks.
                        slippy_tile_download_tasks.0.insert(
                            (SlippyTileCoordinates { x, y }, zoom_level, tile_size),
                            thread,
                        );

                        // Map tile file already exists - add to our map tiles.
                        // Have to re-define filename here because it was already moved into the thread above.
                        let filename = format!(
                            "{}{}.{}.{}.tile.png",
                            filename_string,
                            zoom_level.to_u8(),
                            spc.x,
                            spc.y
                        );
                        slippy_tile_download_status.0.insert(
                            (spc, zoom_level, tile_size),
                            TileDownloadStatus {
                                path: Path::new(&filename).to_path_buf(),
                                load_status: DownloadStatus::Downloaded,
                            },
                        );
                    }
                }
            }
        }
    }
}

pub fn download_slippy_tiles_completed(
    mut slippy_tile_download_status: ResMut<SlippyTileDownloadStatus>,
    mut slippy_tile_download_tasks: ResMut<SlippyTileDownloadTasks>,
    mut slippy_tile_downloaded_events: EventWriter<SlippyTileDownloadedEvent>,
) {
    let mut to_be_removed: Vec<(SlippyTileCoordinates, ZoomLevel, TileSize)> = Vec::new();
    for ((stc, zoom_level, tile_size), task) in slippy_tile_download_tasks.0.iter_mut() {
        if let Some(SlippyTileDownloadTaskResult { path }) =
            future::block_on(future::poll_once(task))
        {
            debug!("Done fetching map tile: {:?}", path);
            // Add to our map tiles.
            slippy_tile_download_status.0.insert(
                (*stc, *zoom_level, *tile_size),
                TileDownloadStatus {
                    path: path.clone(),
                    load_status: DownloadStatus::Downloaded,
                },
            );
            // Notify any event consumers.
            slippy_tile_downloaded_events.send(SlippyTileDownloadedEvent {
                zoom_level: *zoom_level,
                tile_size: *tile_size,
                coordinates: Coordinates::from_slippy_tile_coordinates(stc.x, stc.y),
                path: path.clone(),
            });

            // Task is complete, remove entry.
            to_be_removed.push((*stc, *zoom_level, *tile_size));
        }
    }
    // Clean up finished handled tasks.
    for remove_key in to_be_removed {
        slippy_tile_download_tasks.0.remove(&remove_key);
    }
}
