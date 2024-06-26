use bevy::{
    asset::{
        io::{AssetReaderError, AssetSourceId},
        AssetServer, AsyncWriteExt as _,
    },
    ecs::event::EventReader,
    prelude::{debug, EventWriter, Res, ResMut},
    tasks::{futures_lite::future, IoTaskPool, Task},
};
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
    asset_server: Res<AssetServer>,
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
                    &asset_server,
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
    asset_server: &AssetServer,
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
    let file_exists = future::block_on(does_file_exist(asset_server, &filename));
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
                asset_server,
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

async fn does_file_exist(asset_server: &AssetServer, filename: &str) -> bool {
    let asset_source = asset_server.get_source(AssetSourceId::Default).unwrap();
    let asset_reader = asset_source.reader();
    match asset_reader.read(Path::new(filename)).await {
        Ok(_) => true,
        Err(AssetReaderError::NotFound(_)) => false,
        Err(err) => panic!("failed to check if the file {} exists: {:?}", filename, err),
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
) {
    let task = download_slippy_tile(
        spc,
        zoom_level,
        tile_size,
        endpoint,
        filename.clone(),
        asset_server,
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

fn download_slippy_tile(
    spc: SlippyTileCoordinates,
    zoom_level: ZoomLevel,
    tile_size: TileSize,
    endpoint: String,
    filename: String,
    asset_server: &AssetServer,
) -> Task<SlippyTileDownloadTaskResult> {
    debug!(
        "Fetching map tile at position {:?} with zoom level {:?} from {:?}",
        spc, zoom_level, endpoint
    );
    let tile_url = get_tile_url(endpoint, tile_size, zoom_level, spc.x, spc.y);
    spawn_slippy_tile_download_task(tile_url, filename, asset_server)
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
) -> Task<SlippyTileDownloadTaskResult> {
    let thread_pool = IoTaskPool::get();
    let asset_server = asset_server.clone();
    thread_pool.spawn(async move {
        let request = ehttp::Request {
            method: "GET".to_owned(),
            url: tile_url,
            body: vec![],
            headers: ehttp::Headers::new(&[("User-Agent", "bevy_slippy_tiles")]),
        };
        let response = ehttp::fetch_async(request)
            .await
            .expect("Failed to fetch tile image");
        let asset_source = asset_server.get_source(AssetSourceId::Default).unwrap();
        let asset_writer = asset_source.writer().unwrap();
        let mut writer = asset_writer.write(Path::new(&filename)).await.unwrap();
        writer.write_all(&response.bytes).await.unwrap();
        writer.close().await.unwrap();
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
