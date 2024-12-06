use bevy::prelude::*;
use bevy_slippy_tiles::{
    Coordinates, DownloadSlippyTilesEvent, Radius, SlippyTilesPlugin, SlippyTilesSettings,
    TileSize, ZoomLevel,
};

const LATITUDE: f64 = 45.4111;
const LONGITUDE: f64 = -75.6980;

fn main() {
    App::new()
        .insert_resource(SlippyTilesSettings {
            reference_latitude: LATITUDE,
            reference_longitude: LONGITUDE,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugins(SlippyTilesPlugin)
        .add_systems(Startup, request_slippy_tiles)
        .run();
}

fn request_slippy_tiles(
    mut commands: Commands,
    mut download_slippy_tile_events: EventWriter<DownloadSlippyTilesEvent>,
) {
    commands.spawn(Camera2d::default());
    info!(
        "Requesting slippy tile for latitude/longitude: {:?}",
        (LATITUDE, LONGITUDE)
    );
    let slippy_tile_event = DownloadSlippyTilesEvent {
        tile_size: TileSize::Normal, // Size of tiles - Normal = 256px, Large = 512px (not all tile servers).
        zoom_level: ZoomLevel::L18, // Map zoom level (L0 = entire world, L19 = closest zoom level).
        coordinates: Coordinates::from_latitude_longitude(LATITUDE, LONGITUDE),
        radius: Radius(2), // Request one layer of surrounding tiles (2 = two layers of surrounding tiles - 25 total, 3 = three layers of surrounding tiles - 49 total, etc).
        use_cache: true, // Don't make request if already requested previously, or if file already exists in tiles directory.
    };
    download_slippy_tile_events.send(slippy_tile_event);
}
