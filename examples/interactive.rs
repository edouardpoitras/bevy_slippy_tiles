//! Interactive example demonstrating map panning and coordinate conversion functionality.
//!
//! This example showcases:
//! - Map initialization centered on specific coordinates (Ottawa, Canada)
//! - Interactive panning using left mouse button drag
//! - Coordinate conversion on right mouse button click
//! - Tile downloading with caching support
//! - Zoom in/out with mouse wheel

use bevy::{
    input::{mouse::MouseButton, mouse::MouseWheel, ButtonInput},
    prelude::*,
    window::PrimaryWindow,
};
use bevy_slippy_tiles::{
    world_coords_to_world_pixel, world_pixel_to_world_coords, Coordinates,
    DownloadSlippyTilesMessage, MapTile, Radius, SlippyTilesPlugin, SlippyTilesSettings, TileSize,
    ZoomLevel,
};

/// Default latitude for the map center (Ottawa, Canada)
const LATITUDE: f64 = 45.4111;
/// Default longitude for the map center (Ottawa, Canada)
const LONGITUDE: f64 = -75.6980;
/// Minimum time (in seconds) between zoom operations
const ZOOM_COOLDOWN: f32 = 1.0;

/// Resource to track the state of map panning operations
#[derive(Resource, Default)]
struct PanState {
    /// Whether the user is currently panning the map
    is_panning: bool,
    /// The last recorded cursor position during panning
    last_cursor_position: Option<Vec2>,
}

/// Resource to track the current zoom level and cooldown
#[derive(Resource)]
struct CurrentZoom {
    level: ZoomLevel,
    changed: bool,       // Track if zoom level has changed
    last_zoom_time: f32, // Time of last zoom operation
}

impl Default for CurrentZoom {
    fn default() -> Self {
        Self {
            level: ZoomLevel::L16,
            changed: false,
            last_zoom_time: 0.0,
        }
    }
}

/// Sets up the application with necessary plugins and systems
fn main() {
    App::new()
        .insert_resource(SlippyTilesSettings {
            reference_latitude: LATITUDE,
            reference_longitude: LONGITUDE,
            max_concurrent_downloads: 25, // Increased to handle zoom changes better
            ..Default::default()
        })
        .init_resource::<PanState>()
        .init_resource::<CurrentZoom>()
        .add_plugins(DefaultPlugins)
        .add_plugins(SlippyTilesPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, (pan_camera, handle_click, handle_zoom))
        .add_systems(Update, cleanup_tiles.after(handle_zoom))
        .run();
}

/// Initial setup system that spawns the camera and requests the initial map tiles
///
/// This system:
/// 1. Creates a 2D camera for viewing the map
/// 2. Requests map tiles centered on the specified latitude/longitude
/// 3. Sets up a radius of tiles around the center point for better coverage
fn setup(
    mut commands: Commands,
    mut download_slippy_tile_events: MessageWriter<DownloadSlippyTilesMessage>,
    current_zoom: Res<CurrentZoom>,
) {
    commands.spawn(Camera2d::default());

    info!(
        "Requesting slippy tile for latitude/longitude: {:?}",
        (LATITUDE, LONGITUDE)
    );

    let slippy_tile_event = DownloadSlippyTilesMessage {
        tile_size: TileSize::Normal,
        zoom_level: current_zoom.level,
        coordinates: Coordinates::from_latitude_longitude(LATITUDE, LONGITUDE),
        radius: Radius(2),
        use_cache: true,
    };
    download_slippy_tile_events.write(slippy_tile_event);
}

/// System handling map panning functionality using the left mouse button
///
/// The panning system:
/// - Initiates panning when left mouse button is pressed
/// - Updates the camera position based on cursor movement while panning
/// - Stops panning when left mouse button is released
fn pan_camera(
    mut pan_state: ResMut<PanState>,
    mut camera_query: Query<&mut Transform, With<Camera>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    q_window: Query<&Window, With<PrimaryWindow>>,
) {
    let window = q_window.single().unwrap();

    // Start panning when left mouse button is pressed
    if mouse_button.just_pressed(MouseButton::Left) {
        pan_state.is_panning = true;
        if let Some(position) = window.cursor_position() {
            pan_state.last_cursor_position = Some(position);
        }
    }

    // Stop panning when left mouse button is released
    if mouse_button.just_released(MouseButton::Left) {
        pan_state.is_panning = false;
        pan_state.last_cursor_position = None;
    }

    // If we're panning and have cursor movement
    if pan_state.is_panning {
        let mut camera_transform = camera_query.single_mut().unwrap();

        if let Some(current_position) = window.cursor_position() {
            if let Some(last_position) = pan_state.last_cursor_position {
                let delta = current_position - last_position;
                camera_transform.translation.x -= delta.x;
                camera_transform.translation.y += delta.y;
            }
            pan_state.last_cursor_position = Some(current_position);
        }
    }
}

/// System to clean up tiles when zoom level changes
fn cleanup_tiles(
    mut commands: Commands,
    tile_query: Query<Entity, With<MapTile>>,
    mut current_zoom: ResMut<CurrentZoom>,
) {
    if current_zoom.changed {
        // Despawn all existing tiles
        for entity in tile_query.iter() {
            commands.entity(entity).despawn();
        }
        current_zoom.changed = false;
    }
}

/// System handling mouse wheel input for zooming
fn handle_zoom(
    mut mouse_wheel: MessageReader<MouseWheel>,
    mut current_zoom: ResMut<CurrentZoom>,
    mut download_slippy_tile_events: MessageWriter<DownloadSlippyTilesMessage>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    settings: Res<SlippyTilesSettings>,
    time: Res<Time>,
) {
    // Check if enough time has passed since the last zoom operation
    if time.elapsed_secs() - current_zoom.last_zoom_time < ZOOM_COOLDOWN {
        return;
    }

    // Only process the first wheel event in the queue
    if let Some(event) = mouse_wheel.read().next() {
        let zoom_delta = event.y.signum() as i32;

        // Get current zoom level as u8
        let current_level = current_zoom.level.to_u8();

        // Calculate new zoom level, clamped between 14 and 19
        let new_level = (current_level as i32 + zoom_delta).clamp(14, 19) as u8;

        // Only proceed if zoom level actually changed
        if new_level != current_level {
            // Convert cursor position to world coordinates before zoom
            if let Some(cursor_pos) = q_window.single().unwrap().cursor_position() {
                let (camera, camera_transform) = camera_query.single().unwrap();
                if let Some(cursor_world_pos) = camera
                    .viewport_to_world_2d(camera_transform, cursor_pos)
                    .ok()
                {
                    // Get the reference point's pixel coordinates
                    let (ref_x, ref_y) = world_coords_to_world_pixel(
                        &bevy_slippy_tiles::LatitudeLongitudeCoordinates {
                            latitude: settings.reference_latitude,
                            longitude: settings.reference_longitude,
                        },
                        TileSize::Normal,
                        current_zoom.level,
                    );

                    // Convert cursor position to lat/lon
                    let offset = settings
                        .transform_offset
                        .map_or(Vec3::ZERO, |t| t.translation);
                    let adjusted_position = cursor_world_pos + offset.truncate();
                    let world_coords = world_pixel_to_world_coords(
                        adjusted_position.x as f64 + ref_x,
                        adjusted_position.y as f64 + ref_y,
                        TileSize::Normal,
                        current_zoom.level,
                    );

                    // Update zoom level
                    current_zoom.level = match new_level {
                        14 => ZoomLevel::L14,
                        15 => ZoomLevel::L15,
                        16 => ZoomLevel::L16,
                        17 => ZoomLevel::L17,
                        18 => ZoomLevel::L18,
                        19 => ZoomLevel::L19,
                        _ => current_zoom.level,
                    };
                    current_zoom.changed = true;
                    current_zoom.last_zoom_time = time.elapsed_secs();

                    // Request new tiles at the new zoom level
                    let slippy_tile_event = DownloadSlippyTilesMessage {
                        tile_size: TileSize::Normal,
                        zoom_level: current_zoom.level,
                        coordinates: Coordinates::from_latitude_longitude(
                            world_coords.latitude,
                            world_coords.longitude,
                        ),
                        radius: Radius(2),
                        use_cache: true,
                    };
                    download_slippy_tile_events.write(slippy_tile_event);
                }
            }
        }
    }
}

/// System handling right-click events for coordinate conversion demonstration
///
/// When right-clicking on the map, this system:
/// 1. Converts screen coordinates to world coordinates
/// 2. Adjusts for the reference point offset
/// 3. Converts the position to latitude/longitude
/// 4. Logs the various coordinate representations (screen, map, world)
fn handle_click(
    camera_query: Query<(&Camera, &GlobalTransform)>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    settings: Res<SlippyTilesSettings>,
    current_zoom: Res<CurrentZoom>,
) {
    if mouse_button.just_pressed(MouseButton::Right) {
        let (camera, camera_transform) = camera_query.single().unwrap();
        let window = q_window.single().unwrap();

        if let Some(cursor_position) = window.cursor_position() {
            if let Some(world_2d_position) = camera
                .viewport_to_world_2d(camera_transform, cursor_position)
                .ok()
            {
                // Convert map position to world coordinates considering the reference point and offset
                let offset = settings
                    .transform_offset
                    .map_or(Vec3::ZERO, |t| t.translation);
                let adjusted_position = world_2d_position + offset.truncate();

                // Get the reference point's pixel coordinates
                let (ref_x, ref_y) = world_coords_to_world_pixel(
                    &bevy_slippy_tiles::LatitudeLongitudeCoordinates {
                        latitude: settings.reference_latitude,
                        longitude: settings.reference_longitude,
                    },
                    TileSize::Normal,
                    current_zoom.level,
                );

                // Convert to lat/lon using the current zoom level and tile size, adjusting for the reference point
                let world_coords = world_pixel_to_world_coords(
                    adjusted_position.x as f64 + ref_x,
                    adjusted_position.y as f64 + ref_y,
                    TileSize::Normal,
                    current_zoom.level,
                );

                info!(
                    "Clicked:\nScreen: {} x {}\nMap: {}, {}\nWorld: lat {} lon {}\nZoom: {}",
                    cursor_position.x,
                    cursor_position.y,
                    world_2d_position.x,
                    world_2d_position.y,
                    world_coords.latitude,
                    world_coords.longitude,
                    current_zoom.level.to_u8()
                );
            }
        }
    }
}
