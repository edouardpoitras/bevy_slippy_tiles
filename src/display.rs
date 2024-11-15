use crate::{
    world_coords_to_world_pixel, LatitudeLongitudeCoordinates, SlippyTileDownloadedEvent,
    SlippyTilesSettings,
};
use bevy::prelude::*;

/// Component to mark entities as map tiles
#[derive(Component)]
pub struct MapTile;

/// System to display tiles as they are downloaded
pub fn display_tiles(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    settings: Res<SlippyTilesSettings>,
    mut tile_events: EventReader<SlippyTileDownloadedEvent>,
) {
    // Skip if auto-render is disabled
    if !settings.auto_render {
        return;
    }

    for event in tile_events.read() {
        // Convert reference coordinates to pixel coordinates
        let reference_point = LatitudeLongitudeCoordinates {
            latitude: settings.reference_latitude,
            longitude: settings.reference_longitude,
        };
        let (ref_x, ref_y) =
            world_coords_to_world_pixel(&reference_point, event.tile_size, event.zoom_level);

        // Convert tile coordinates to pixel coordinates
        let current_coords = match event.coordinates {
            crate::Coordinates::LatitudeLongitude(coords) => coords,
            crate::Coordinates::SlippyTile(coords) => {
                coords.to_latitude_longitude(event.zoom_level)
            },
        };
        let (tile_x, tile_y) =
            world_coords_to_world_pixel(&current_coords, event.tile_size, event.zoom_level);

        // Calculate offset from reference point
        let mut transform_x = (tile_x - ref_x) as f32;
        let mut transform_y = (tile_y - ref_y) as f32;

        // Apply optional transform offset
        if let Some(offset) = &settings.transform_offset {
            transform_x += offset.translation.x;
            transform_y += offset.translation.y;
        }

        // Spawn the tile sprite
        commands.spawn((
            SpriteBundle {
                texture: asset_server.load(event.path.clone()),
                transform: Transform::from_xyz(transform_x, transform_y, settings.z_layer),
                ..default()
            },
            MapTile,
        ));
    }
}
