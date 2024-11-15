//! Interactive example demonstrating map panning and coordinate conversion functionality.
//!
//! This example showcases:
//! - Map initialization centered on specific coordinates (Ottawa, Canada)
//! - Interactive panning using left mouse button drag
//! - Coordinate conversion on right mouse button click
//! - Tile downloading with caching support

use bevy::{
    input::{mouse::MouseButton, ButtonInput},
    prelude::*,
    window::PrimaryWindow,
};
use bevy_slippy_tiles::{
    Coordinates, DownloadSlippyTilesEvent, Radius, SlippyTilesPlugin, SlippyTilesSettings,
    TileSize, ZoomLevel, world_pixel_to_world_coords, world_coords_to_world_pixel,
};

/// Default latitude for the map center (Ottawa, Canada)
const LATITUDE: f64 = 45.4111;
/// Default longitude for the map center (Ottawa, Canada)
const LONGITUDE: f64 = -75.6980;

/// Resource to track the state of map panning operations
#[derive(Resource, Default)]
struct PanState {
    /// Whether the user is currently panning the map
    is_panning: bool,
    /// The last recorded cursor position during panning
    last_cursor_position: Option<Vec2>,
}

/// Sets up the application with necessary plugins and systems
fn main() {
    App::new()
        .insert_resource(SlippyTilesSettings {
            reference_latitude: LATITUDE,
            reference_longitude: LONGITUDE,
            ..Default::default()
        })
        .init_resource::<PanState>()
        .add_plugins(DefaultPlugins)
        .add_plugins(SlippyTilesPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, (pan_camera, handle_click))
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
    mut download_slippy_tile_events: EventWriter<DownloadSlippyTilesEvent>,
) {
    commands.spawn(Camera2dBundle::default());
    
    info!(
        "Requesting slippy tile for latitude/longitude: {:?}",
        (LATITUDE, LONGITUDE)
    );
    
    let slippy_tile_event = DownloadSlippyTilesEvent {
        tile_size: TileSize::Normal,
        zoom_level: ZoomLevel::L18,
        coordinates: Coordinates::from_latitude_longitude(LATITUDE, LONGITUDE),
        radius: Radius(2),
        use_cache: true,
    };
    download_slippy_tile_events.send(slippy_tile_event);
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
    let window = q_window.single();

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
        let mut camera_transform = camera_query.single_mut();
        
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
) {
    if mouse_button.just_pressed(MouseButton::Right) {
        let (camera, camera_transform) = camera_query.single();
        let window = q_window.single();
        
        if let Some(cursor_position) = window.cursor_position() {
            if let Some(world_2d_position) = camera.viewport_to_world_2d(camera_transform, cursor_position) {
                // Convert map position to world coordinates considering the reference point and offset
                let offset = settings.transform_offset.map_or(Vec3::ZERO, |t| t.translation);
                let adjusted_position = world_2d_position + offset.truncate();
                
                // Get the reference point's pixel coordinates
                let (ref_x, ref_y) = world_coords_to_world_pixel(
                    &bevy_slippy_tiles::LatitudeLongitudeCoordinates {
                        latitude: settings.reference_latitude,
                        longitude: settings.reference_longitude,
                    },
                    TileSize::Normal,
                    ZoomLevel::L18,
                );

                // Convert to lat/lon using the current zoom level and tile size, adjusting for the reference point
                let world_coords = world_pixel_to_world_coords(
                    adjusted_position.x as f64 + ref_x,
                    adjusted_position.y as f64 + ref_y,
                    TileSize::Normal,
                    ZoomLevel::L18,
                );

                info!("Clicked:\nScreen: {} x {}\nMap: {}, {}\nWorld: lat {} lon {}",
                    cursor_position.x, cursor_position.y, world_2d_position.x, world_2d_position.y, world_coords.latitude, world_coords.longitude
                );
            }
        }
    }
}
