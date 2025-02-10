use bevy::{
    asset::{
        io::{AssetReaderError, AssetSourceId},
        AssetServer, AsyncWriteExt as _,
    },
    ecs::event::EventReader,
    prelude::{debug, warn, EventWriter, Res, ResMut, Resource},
    tasks::{futures_lite::future, IoTaskPool, Task},
};
use std::{
    collections::VecDeque,
    path::Path,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Instant,
};

use crate::{
    AlreadyDownloaded, Coordinates, DownloadSlippyTilesEvent, DownloadStatus, FileExists,
    SlippyTileCoordinates, SlippyTileDownloadStatus, SlippyTileDownloadTaskKey,
    SlippyTileDownloadTaskResult, SlippyTileDownloadTasks, SlippyTileDownloadedEvent,
    SlippyTilesSettings, TileDownloadStatus, TileSize, UseCache, ZoomLevel,
};

#[derive(Debug)]
struct BufferedRequest {
    coords: (u32, u32),
    zoom_level: ZoomLevel,
    tile_size: TileSize,
    endpoint: String,
    filename: String,
}

#[derive(Resource, Default)]
pub struct DownloadRateLimiter {
    requests: VecDeque<Instant>,
    buffered_requests: VecDeque<BufferedRequest>,
}

impl DownloadRateLimiter {
    fn can_make_request(&mut self, now: Instant, settings: &SlippyTilesSettings) -> bool {
        // Remove old requests outside the window
        while let Some(time) = self.requests.front() {
            if now.duration_since(*time) > settings.rate_limit_window {
                self.requests.pop_front();
            } else {
                break;
            }
        }

        // Check if we can make a new request
        if self.requests.len() < settings.rate_limit_requests {
            self.requests.push_back(now);
            true
        } else {
            false
        }
    }

    fn buffer_request(
        &mut self,
        coords: (u32, u32),
        zoom_level: ZoomLevel,
        tile_size: TileSize,
        endpoint: String,
        filename: String,
    ) {
        self.buffered_requests.push_back(BufferedRequest {
            coords,
            zoom_level,
            tile_size,
            endpoint,
            filename,
        });
    }

    fn process_buffered_requests(
        &mut self,
        slippy_tile_download_tasks: &mut ResMut<SlippyTileDownloadTasks>,
        slippy_tile_download_status: &mut ResMut<SlippyTileDownloadStatus>,
        asset_server: &AssetServer,
        active_downloads: &ActiveDownloads,
        settings: &SlippyTilesSettings,
    ) {
        let now = Instant::now();
        while self.can_make_request(now, settings) {
            if let Some(request) = self.buffered_requests.pop_front() {
                let spc = SlippyTileCoordinates {
                    x: request.coords.0,
                    y: request.coords.1,
                };

                download_and_track_slippy_tile(
                    spc,
                    request.zoom_level,
                    request.tile_size,
                    request.endpoint,
                    request.filename,
                    slippy_tile_download_tasks,
                    slippy_tile_download_status,
                    asset_server,
                    active_downloads,
                    settings,
                );
            } else {
                break;
            }
        }
    }
}

#[derive(Resource)]
pub struct ActiveDownloads(Arc<AtomicUsize>);

impl Default for ActiveDownloads {
    fn default() -> Self {
        Self(Arc::new(AtomicUsize::new(0)))
    }
}

/// System that listens for DownloadSlippyTiles events and submits individual tile requests in separate threads.
pub fn download_slippy_tiles(
    mut download_slippy_tile_events: EventReader<DownloadSlippyTilesEvent>,
    slippy_tiles_settings: Res<SlippyTilesSettings>,
    mut slippy_tile_download_status: ResMut<SlippyTileDownloadStatus>,
    mut slippy_tile_download_tasks: ResMut<SlippyTileDownloadTasks>,
    mut rate_limiter: ResMut<DownloadRateLimiter>,
    active_downloads: Res<ActiveDownloads>,
    asset_server: Res<AssetServer>,
) {
    // First process any buffered requests
    rate_limiter.process_buffered_requests(
        &mut slippy_tile_download_tasks,
        &mut slippy_tile_download_status,
        &asset_server,
        &active_downloads,
        &slippy_tiles_settings,
    );

    for download_slippy_tile in download_slippy_tile_events.read() {
        let radius = download_slippy_tile.radius.0;
        let slippy_tile_coords = download_slippy_tile.get_slippy_tile_coordinates();

        // Calculate tile range with overflow protection
        let min_x = slippy_tile_coords.x.saturating_sub(radius as u32);
        let min_y = slippy_tile_coords.y.saturating_sub(radius as u32);
        let max_x = slippy_tile_coords.x.saturating_add(radius as u32);
        let max_y = slippy_tile_coords.y.saturating_add(radius as u32);

        for x in min_x..=max_x {
            for y in min_y..=max_y {
                // Check concurrent download limit
                if active_downloads.0.load(Ordering::Relaxed)
                    >= slippy_tiles_settings.max_concurrent_downloads
                {
                    warn!("Max concurrent downloads reached, buffering tile download");
                    rate_limiter.buffer_request(
                        (x, y),
                        download_slippy_tile.zoom_level,
                        download_slippy_tile.tile_size,
                        slippy_tiles_settings.endpoint.clone(),
                        get_tile_filename(
                            slippy_tiles_settings.get_tiles_directory_string(),
                            download_slippy_tile.zoom_level,
                            x,
                            y,
                            download_slippy_tile.tile_size,
                        ),
                    );
                    continue;
                }

                let spc = SlippyTileCoordinates { x, y };
                let tiles_directory = slippy_tiles_settings.get_tiles_directory_string();
                let filename = get_tile_filename(
                    tiles_directory,
                    download_slippy_tile.zoom_level,
                    x,
                    y,
                    download_slippy_tile.tile_size,
                );

                let already_downloaded = slippy_tile_download_status.contains_key_with_coords(
                    spc,
                    download_slippy_tile.zoom_level,
                    download_slippy_tile.tile_size,
                );

                let file_exists = async_file_exists(&asset_server, &filename);

                match (
                    UseCache::new(download_slippy_tile.use_cache),
                    AlreadyDownloaded::new(already_downloaded),
                    FileExists::new(file_exists),
                ) {
                    // This should only match when waiting on a file download.
                    (_, AlreadyDownloaded::Yes, FileExists::No) => {
                        // Check if the download has timed out
                        if let Some(status) = slippy_tile_download_status.0.get(&SlippyTileDownloadTaskKey {
                            slippy_tile_coordinates: spc,
                            zoom_level: download_slippy_tile.zoom_level,
                            tile_size: download_slippy_tile.tile_size,
                        }) {
                            if matches!(status.load_status, DownloadStatus::Downloading) {
                                rate_limiter.buffer_request(
                                    (x, y),
                                    download_slippy_tile.zoom_level,
                                    download_slippy_tile.tile_size,
                                    slippy_tiles_settings.endpoint.clone(),
                                    filename,
                                );
                            }
                        }
                    }
                    // Cache can not be used,
                    (UseCache::No, _, _)
                    // OR not downloading yet and no file exists on disk.
                    | (UseCache::Yes, AlreadyDownloaded::No, FileExists::No) => {
                        rate_limiter.buffer_request(
                            (x, y),
                            download_slippy_tile.zoom_level,
                            download_slippy_tile.tile_size,
                            slippy_tiles_settings.endpoint.clone(),
                            filename,
                        );
                    }
                    // Cache can be used and we have the file on disk.
                    (UseCache::Yes, _, FileExists::Yes) => load_and_track_slippy_tile_from_disk(
                        spc,
                        download_slippy_tile.zoom_level,
                        download_slippy_tile.tile_size,
                        filename,
                        &mut slippy_tile_download_tasks,
                        &mut slippy_tile_download_status,
                    ),
                }
            }
        }
    }
}

fn get_tile_filename(
    tiles_directory: String,
    zoom_level: ZoomLevel,
    x: u32,
    y: u32,
    tile_size: TileSize,
) -> String {
    format!(
        "{}{}.{}.{}.{}.tile.png",
        tiles_directory,
        zoom_level.to_u8(),
        x,
        y,
        tile_size.to_pixels()
    )
}

fn async_file_exists(asset_server: &AssetServer, filename: &str) -> bool {
    let asset_source = match asset_server.get_source(AssetSourceId::Default) {
        Ok(source) => source,
        Err(_) => return false,
    };

    let asset_reader = asset_source.reader();
    match future::block_on(asset_reader.read(Path::new(filename))) {
        Ok(_) => true,
        Err(AssetReaderError::NotFound(_)) => false,
        Err(_) => false,
    }
}

#[allow(clippy::too_many_arguments)]
fn download_and_track_slippy_tile(
    spc: SlippyTileCoordinates,
    zoom_level: ZoomLevel,
    tile_size: TileSize,
    endpoint: String,
    filename: String,
    slippy_tile_download_tasks: &mut ResMut<SlippyTileDownloadTasks>,
    slippy_tile_download_status: &mut ResMut<SlippyTileDownloadStatus>,
    asset_server: &AssetServer,
    active_downloads: &ActiveDownloads,
    settings: &SlippyTilesSettings,
) {
    let task = download_slippy_tile(
        spc,
        zoom_level,
        tile_size,
        endpoint,
        filename.clone(),
        asset_server,
        active_downloads.0.clone(),
        settings.max_retries,
    );

    slippy_tile_download_tasks.insert(spc.x, spc.y, zoom_level, tile_size, task);
    slippy_tile_download_status.insert_with_coords(
        spc,
        zoom_level,
        tile_size,
        filename,
        DownloadStatus::Downloading,
    );
}

#[allow(clippy::too_many_arguments)]
fn download_slippy_tile(
    spc: SlippyTileCoordinates,
    zoom_level: ZoomLevel,
    tile_size: TileSize,
    endpoint: String,
    filename: String,
    asset_server: &AssetServer,
    active_downloads: Arc<AtomicUsize>,
    max_retries: u32,
) -> Task<SlippyTileDownloadTaskResult> {
    debug!(
        "Fetching map tile at position {:?} with zoom level {:?} from {:?}",
        spc, zoom_level, endpoint
    );
    let tile_url = get_tile_url(endpoint, tile_size, zoom_level, spc.x, spc.y);
    spawn_slippy_tile_download_task(
        tile_url,
        filename,
        asset_server,
        active_downloads,
        max_retries,
    )
}

fn get_tile_url(
    endpoint: String,
    tile_size: TileSize,
    zoom_level: ZoomLevel,
    x: u32,
    y: u32,
) -> String {
    format!(
        "{}/{}/{}/{}{}.png",
        endpoint,
        zoom_level.to_u8(),
        x,
        y,
        tile_size.get_url_postfix()
    )
}

fn spawn_slippy_tile_download_task(
    tile_url: String,
    filename: String,
    asset_server: &AssetServer,
    active_downloads: Arc<AtomicUsize>,
    max_retries: u32,
) -> Task<SlippyTileDownloadTaskResult> {
    let thread_pool = IoTaskPool::get();
    let asset_server = asset_server.clone();

    active_downloads.fetch_add(1, Ordering::SeqCst);

    thread_pool.spawn(async move {
        let mut retries = 0;
        let result = loop {
            if retries >= max_retries {
                warn!("Max retries reached for tile download: {}", tile_url);
                break Err("Max retries reached".to_string());
            }

            let request = ehttp::Request {
                method: "GET".to_owned(),
                url: tile_url.clone(),
                body: vec![],
                headers: ehttp::Headers::new(&[
                    ("User-Agent", "bevy_slippy_tiles/0.7.0 (https://github.com/edouardpoitras/bevy_slippy_tiles)"),
                    ("Accept", "image/png"),
                ]),
            };

            match ehttp::fetch_async(request).await {
                Ok(response) => {
                    if response.status == 200 {
                        let asset_source = asset_server.get_source(AssetSourceId::Default).unwrap();
                        let asset_writer = match asset_source.writer() {
                            Ok(writer) => writer,
                            Err(e) => {
                                warn!("Failed to get asset writer: {:?}", e);
                                retries += 1;
                                continue;
                            }
                        };

                        let mut writer = match asset_writer.write(Path::new(&filename)).await {
                            Ok(writer) => writer,
                            Err(e) => {
                                warn!("Failed to create file writer: {:?}", e);
                                retries += 1;
                                continue;
                            }
                        };

                        if let Err(e) = writer.write_all(&response.bytes).await {
                            warn!("Failed to write tile data: {:?}", e);
                            retries += 1;
                            continue;
                        }

                        if let Err(e) = writer.close().await {
                            warn!("Failed to close file writer: {:?}", e);
                            retries += 1;
                            continue;
                        }

                        break Ok(());
                    } else {
                        warn!("HTTP error {}: {}", response.status, response.status_text);
                        retries += 1;
                        continue;
                    }
                }
                Err(e) => {
                    warn!("Download error: {:?}", e);
                    retries += 1;
                    continue;
                }
            }
        };

        active_downloads.fetch_sub(1, Ordering::SeqCst);

        match result {
            Ok(()) => SlippyTileDownloadTaskResult {
                path: Path::new(&filename).to_path_buf(),
            },
            Err(e) => {
                warn!("Failed to download tile: {}", e);
                SlippyTileDownloadTaskResult {
                    path: Path::new(&filename).to_path_buf(),
                }
            }
        }
    })
}

fn load_and_track_slippy_tile_from_disk(
    spc: SlippyTileCoordinates,
    zoom_level: ZoomLevel,
    tile_size: TileSize,
    filename: String,
    slippy_tile_download_tasks: &mut ResMut<SlippyTileDownloadTasks>,
    slippy_tile_download_status: &mut ResMut<SlippyTileDownloadStatus>,
) {
    let task = load_slippy_tile_from_disk(filename.clone());
    slippy_tile_download_tasks.insert_with_coords(spc, zoom_level, tile_size, task);
    slippy_tile_download_status.insert_with_coords(
        spc,
        zoom_level,
        tile_size,
        filename,
        DownloadStatus::Downloaded,
    );
}

fn load_slippy_tile_from_disk(filename: String) -> Task<SlippyTileDownloadTaskResult> {
    debug!("Loading slippy tile from disk - {}", filename);
    spawn_fake_slippy_tile_download_task(filename)
}

fn spawn_fake_slippy_tile_download_task(filename: String) -> Task<SlippyTileDownloadTaskResult> {
    let thread_pool = IoTaskPool::get();
    thread_pool.spawn(async move {
        SlippyTileDownloadTaskResult {
            path: Path::new(&filename).to_path_buf(),
        }
    })
}

/// System that checks for completed slippy tile downloads and notifies via a SlippyTileDownloadedEvent event.
pub fn download_slippy_tiles_completed(
    mut slippy_tile_download_status: ResMut<SlippyTileDownloadStatus>,
    mut slippy_tile_download_tasks: ResMut<SlippyTileDownloadTasks>,
    mut slippy_tile_downloaded_events: EventWriter<SlippyTileDownloadedEvent>,
) {
    let mut to_be_removed: Vec<SlippyTileDownloadTaskKey> = Vec::new();
    for (stdtk, task) in slippy_tile_download_tasks.0.iter_mut() {
        if let Some(SlippyTileDownloadTaskResult { path }) =
            future::block_on(future::poll_once(task))
        {
            debug!("Done fetching map tile: {:?}", path);
            // Add to our map tiles.
            slippy_tile_download_status.0.insert(
                stdtk.clone(),
                TileDownloadStatus {
                    path: path.clone(),
                    load_status: DownloadStatus::Downloaded,
                },
            );
            // Notify any event consumers.
            slippy_tile_downloaded_events.send(SlippyTileDownloadedEvent {
                zoom_level: stdtk.zoom_level,
                tile_size: stdtk.tile_size,
                coordinates: Coordinates::from_slippy_tile_coordinates(
                    stdtk.slippy_tile_coordinates.x,
                    stdtk.slippy_tile_coordinates.y,
                ),
                path: path.clone(),
            });
            // Task is complete, remove entry.
            to_be_removed.push(stdtk.clone());
        }
    }
    // Clean up finished handled tasks.
    for remove_key in to_be_removed {
        slippy_tile_download_tasks.0.remove(&remove_key);
    }
}
