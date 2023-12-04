use bevy::{
    ecs::event::EventReader,
    prelude::{debug, EventWriter, Res, ResMut},
    tasks::{IoTaskPool, Task},
};
use futures_lite::future;
use std::path::Path;

use crate::{
    AlreadyDownloaded, Coordinates, DownloadSlippyTilesEvent, DownloadStatus, FileExists,
    SlippyTileCoordinates, SlippyTileDownloadStatus, SlippyTileDownloadTaskKey,
    SlippyTileDownloadTaskResult, SlippyTileDownloadTasks, SlippyTileDownloadedEvent,
    SlippyTilesSettings, TileDownloadStatus, TileSize, UseCache, ZoomLevel,
};

/// System that listens for DownloadSlippyTiles events and submits individual tile requests in separate threads.
pub fn download_slippy_tiles(
    mut download_slippy_tile_events: EventReader<DownloadSlippyTilesEvent>,
    slippy_tiles_settings: Res<SlippyTilesSettings>,
    mut slippy_tile_download_status: ResMut<SlippyTileDownloadStatus>,
    mut slippy_tile_download_tasks: ResMut<SlippyTileDownloadTasks>,
) {
    for download_slippy_tile in download_slippy_tile_events.read() {
        let radius = download_slippy_tile.radius.0;
        let slippy_tile_coords = download_slippy_tile.get_slippy_tile_coordinates();
        for x in slippy_tile_coords.x - radius as u32..slippy_tile_coords.x + radius as u32 + 1 {
            for y in slippy_tile_coords.y - radius as u32..slippy_tile_coords.y + radius as u32 + 1
            {
                handle_download_slippy_tile_event(
                    x,
                    y,
                    download_slippy_tile,
                    &slippy_tiles_settings,
                    &mut slippy_tile_download_tasks,
                    &mut slippy_tile_download_status,
                );
            }
        }
    }
}

fn handle_download_slippy_tile_event(
    x: u32,
    y: u32,
    download_slippy_tile_event: &DownloadSlippyTilesEvent,
    slippy_tiles_settings: &Res<SlippyTilesSettings>,
    slippy_tile_download_tasks: &mut ResMut<SlippyTileDownloadTasks>,
    slippy_tile_download_status: &mut ResMut<SlippyTileDownloadStatus>,
) {
    let spc = SlippyTileCoordinates { x, y };
    let tiles_directory = slippy_tiles_settings.get_tiles_directory_string();
    let filename = get_tile_filename(
        tiles_directory,
        download_slippy_tile_event.zoom_level,
        x,
        y,
        download_slippy_tile_event.tile_size,
    );
    let already_downloaded = slippy_tile_download_status.contains_key_with_coords(
        spc,
        download_slippy_tile_event.zoom_level,
        download_slippy_tile_event.tile_size,
    );
    let file_exists = std::path::Path::new(&format!("assets/{filename}")).exists();
    match (
        UseCache::new(download_slippy_tile_event.use_cache),
        AlreadyDownloaded::new(already_downloaded),
        FileExists::new(file_exists),
    ) {
        // This should only match when waiting on a file download.
        (_, AlreadyDownloaded::Yes, FileExists::No) => {
            // Assume the file will eventually download.
            // TODO: This needs to be more robust.
        },
        // Cache can not be used,
        (UseCache::No, _, _)
        // OR not downloading yet and no file exists on disk.
        | (UseCache::Yes, AlreadyDownloaded::No, FileExists::No) => {
            download_and_track_slippy_tile(
                spc,
                download_slippy_tile_event.zoom_level,
                download_slippy_tile_event.tile_size,
                slippy_tiles_settings.endpoint.clone(),
                filename,
                slippy_tile_download_tasks,
                slippy_tile_download_status,
            );
        },
        // Cache can be used and we have the file on disk.
        (UseCache::Yes, _, FileExists::Yes) => load_and_track_slippy_tile_from_disk(
            spc,
            download_slippy_tile_event.zoom_level,
            download_slippy_tile_event.tile_size,
            filename,
            slippy_tile_download_tasks,
            slippy_tile_download_status,
        ),
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

fn download_and_track_slippy_tile(
    spc: SlippyTileCoordinates,
    zoom_level: ZoomLevel,
    tile_size: TileSize,
    endpoint: String,
    filename: String,
    slippy_tile_download_tasks: &mut ResMut<SlippyTileDownloadTasks>,
    slippy_tile_download_status: &mut ResMut<SlippyTileDownloadStatus>,
) {
    let task = download_slippy_tile(spc, zoom_level, tile_size, endpoint, filename.clone());
    slippy_tile_download_tasks.insert(spc.x, spc.y, zoom_level, tile_size, task);
    slippy_tile_download_status.insert_with_coords(
        spc,
        zoom_level,
        tile_size,
        filename,
        DownloadStatus::Downloading,
    );
}

fn download_slippy_tile(
    spc: SlippyTileCoordinates,
    zoom_level: ZoomLevel,
    tile_size: TileSize,
    endpoint: String,
    filename: String,
) -> Task<SlippyTileDownloadTaskResult> {
    debug!(
        "Fetching map tile at position {:?} with zoom level {:?} from {:?}",
        spc, zoom_level, endpoint
    );
    let tile_url = get_tile_url(endpoint, tile_size, zoom_level, spc.x, spc.y);
    spawn_slippy_tile_download_task(tile_url, filename)
}

fn get_tile_url(
    endpoint: String,
    tile_size: TileSize,
    zoom_level: ZoomLevel,
    x: u32,
    y: u32,
) -> String {
    match tile_size {
        TileSize::Normal => {
            format!("{}/{}/{}/{}.png", endpoint, zoom_level.to_u8(), x, y)
        },
        TileSize::Large => {
            format!("{}/{}/{}/{}@2.png", endpoint, zoom_level.to_u8(), x, y)
        },
    }
}

fn spawn_slippy_tile_download_task(
    tile_url: String,
    filename: String,
) -> Task<SlippyTileDownloadTaskResult> {
    let thread_pool = IoTaskPool::get();
    thread_pool.spawn(async move {
        let client = reqwest::blocking::Client::new();
        let response = client
            .get(tile_url)
            .header(reqwest::header::USER_AGENT, "bevy_slippy_tiles")
            .send()
            .expect("Failed to fetch tile image");
        let mut content = std::io::Cursor::new(
            response
                .bytes()
                .expect("Could not get tile image from bytes"),
        );
        let mut file_out = std::fs::File::create(format!("assets/{filename}")).unwrap();
        std::io::copy(&mut content, &mut file_out).unwrap();
        SlippyTileDownloadTaskResult {
            path: Path::new(&filename).to_path_buf(),
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
