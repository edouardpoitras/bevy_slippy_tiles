use bevy::{ecs::schedule::ShouldRun, prelude::*};
use bevy_slippy_tiles::{
    Coordinates, DownloadSlippyTilesEvent, Radius, SlippyTileCoordinates,
    SlippyTileDownloadedEvent, SlippyTilesPlugin, SlippyTilesSettings, TileSize, ZoomLevel,
};

const LATITUDE: f64 = 45.4111;
const LONGITUDE: f64 = -75.6980;

fn main() {
    App::new()
        // Our slippy tiles settings and plugin
        .insert_resource(SlippyTilesSettings::new(
            "https://tile.openstreetmap.org", // Tile server endpoint.
            "tiles/",                         // assets/ folder storing the slippy tile downloads.
        ))
        .add_plugins(DefaultPlugins)
        .add_plugin(SlippyTilesPlugin)
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(ShouldRun::once)
                .with_system(spawn_camera)
                .with_system(request_slippy_tiles),
        )
        .add_system(display_slippy_tiles)
        .run();
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn request_slippy_tiles(mut download_slippy_tile_events: EventWriter<DownloadSlippyTilesEvent>) {
    info!(
        "Requesting slippy tile for latitude/longitude: {:?}",
        (LATITUDE, LONGITUDE)
    );
    let slippy_tile_event = DownloadSlippyTilesEvent {
        tile_size: TileSize::Normal, // Size of tiles - Normal = 256px, Large = 512px (not all tile servers).
        zoom_level: ZoomLevel::L18, // Map zoom level (L0 = entire world, L19 = closest zoom level).
        coordinates: Coordinates::from_latitude_longitude(LATITUDE, LONGITUDE),
        radius: Radius(1), // Request one layer of surrounding tiles (2 = two layers of surrounding tiles - 25 total, 3 = three layers of surrounding tiles - 49 total, etc).
        use_cache: true, // Don't make request if already requested previously, or if file already exists in tiles directory.
    };
    download_slippy_tile_events.send(slippy_tile_event);
}

fn display_slippy_tiles(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut slippy_tile_downloaded_events: EventReader<SlippyTileDownloadedEvent>,
) {
    for slippy_tile_downloaded_event in slippy_tile_downloaded_events.iter() {
        info!("Slippy tile fetched: {:?}", slippy_tile_downloaded_event);
        let zoom_level = slippy_tile_downloaded_event.zoom_level;
        // Convert our slippy tile position to pixels on the screen relative to the center tile.
        let SlippyTileCoordinates {
            x: center_x,
            y: center_y,
        } = Coordinates::from_latitude_longitude(LATITUDE, LONGITUDE)
            .get_slippy_tile_coordinates(zoom_level);
        let SlippyTileCoordinates {
            x: current_x,
            y: current_y,
        } = slippy_tile_downloaded_event
            .coordinates
            .get_slippy_tile_coordinates(zoom_level);

        let tile_pixels = slippy_tile_downloaded_event.tile_size.to_pixels() as f32;
        let transform_x = (current_x as f32 - center_x as f32) * tile_pixels;
        let transform_y = (center_y as f32 - current_y as f32) * tile_pixels;

        // Add our slippy tile to the screen.
        commands.spawn(SpriteBundle {
            texture: asset_server.load(slippy_tile_downloaded_event.path.clone()),
            transform: Transform::from_xyz(transform_x, transform_y, 0.0),
            ..Default::default()
        });
    }
}
