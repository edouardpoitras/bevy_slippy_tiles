#![doc = include_str!("../README.md")]

mod constants;
mod coordinates;
mod download;
mod settings;
mod systems;
mod types;

pub use constants::*;
pub use coordinates::*;
pub use download::*;
pub use settings::*;
pub use types::*;

use bevy::prelude::{App, Plugin, Update};

pub struct SlippyTilesPlugin;

impl Plugin for SlippyTilesPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SlippyTileDownloadStatus::new())
            .insert_resource(SlippyTileDownloadTasks::new())
            .insert_resource(systems::DownloadRateLimiter::default())
            .insert_resource(systems::ActiveDownloads::default())
            .add_event::<DownloadSlippyTilesEvent>()
            .add_event::<SlippyTileDownloadedEvent>()
            .add_systems(Update, systems::download_slippy_tiles)
            .add_systems(Update, systems::download_slippy_tiles_completed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_size_new() {
        assert_eq!(TileSize::new(256), TileSize::Normal);
        assert_eq!(TileSize::new(512), TileSize::Large);
        assert_eq!(TileSize::new(768), TileSize::VeryLarge);
        assert_eq!(TileSize::new(1024), TileSize::Normal);
    }

    #[test]
    fn test_slippy_tile_settings() {
        let sts = SlippyTilesSettings { endpoint: "endpoint".into(), tiles_directory: "tiles_directory".into(), ..Default::default() };
        assert_eq!(sts.get_endpoint(), "endpoint");
        assert_eq!(
            sts.get_tiles_directory(),
            std::path::PathBuf::from("tiles_directory")
        );
        assert_eq!(sts.get_tiles_directory_string(), "tiles_directory");
        assert!(std::path::Path::try_exists("assets/tiles_directory".as_ref()).is_ok());
    }

    #[test]
    fn test_slippy_tile_coordinates() {
        assert_eq!(
            SlippyTileCoordinates::from_latitude_longitude(85.0511287798066, 0.0, ZoomLevel::L1),
            SlippyTileCoordinates { x: 1, y: 0 }
        );
        assert_eq!(
            SlippyTileCoordinates::to_latitude_longitude(
                &SlippyTileCoordinates { x: 1, y: 0 },
                ZoomLevel::L1
            ),
            LatitudeLongitudeCoordinates {
                latitude: 85.0511287798066,
                longitude: 0.0
            }
        );
        assert_eq!(
            SlippyTileCoordinates::from_latitude_longitude(0.0, 0.0, ZoomLevel::L10),
            SlippyTileCoordinates { x: 512, y: 512 }
        );
        assert_eq!(
            SlippyTileCoordinates::to_latitude_longitude(
                &SlippyTileCoordinates { x: 512, y: 512 },
                ZoomLevel::L10
            ),
            LatitudeLongitudeCoordinates {
                latitude: 0.0,
                longitude: 0.0
            }
        );
        assert_eq!(
            SlippyTileCoordinates::from_latitude_longitude(
                48.81590713080016,
                2.2686767578125,
                ZoomLevel::L17
            ),
            SlippyTileCoordinates { x: 66362, y: 45115 }
        );
        assert_eq!(
            SlippyTileCoordinates::to_latitude_longitude(
                &SlippyTileCoordinates { x: 66362, y: 45115 },
                ZoomLevel::L17
            ),
            LatitudeLongitudeCoordinates {
                latitude: 48.81590713080016,
                longitude: 2.2686767578125
            }
        );
        assert_eq!(
            SlippyTileCoordinates::from_latitude_longitude(
                0.004806518549043551,
                0.004119873046875,
                ZoomLevel::L19
            ),
            SlippyTileCoordinates {
                x: 262150,
                y: 262137
            }
        );
        assert_eq!(
            SlippyTileCoordinates::to_latitude_longitude(
                &SlippyTileCoordinates {
                    x: 262150,
                    y: 262137
                },
                ZoomLevel::L19
            ),
            LatitudeLongitudeCoordinates {
                latitude: 0.004806518549043551,
                longitude: 0.004119873046875
            }
        );
        assert_eq!(
            SlippyTileCoordinates::from_latitude_longitude(
                26.850416392948524,
                72.57980346679688,
                ZoomLevel::L19
            ),
            SlippyTileCoordinates {
                x: 367846,
                y: 221525
            }
        );
        assert_eq!(
            SlippyTileCoordinates::to_latitude_longitude(
                &SlippyTileCoordinates {
                    x: 367846,
                    y: 221525
                },
                ZoomLevel::L19
            ),
            LatitudeLongitudeCoordinates {
                latitude: 26.850416392948524,
                longitude: 72.57980346679688
            }
        );
    }

    #[test]
    fn test_slippy_tile_download_status() {
        let mut stds = SlippyTileDownloadStatus::default();
        stds.insert(
            100,
            50,
            ZoomLevel::L10,
            TileSize::Normal,
            "filename".into(),
            DownloadStatus::Downloading,
        );
        assert!(!stds.contains_key(100, 50, ZoomLevel::L1, TileSize::Normal));
        assert!(!stds.contains_key(100, 50, ZoomLevel::L10, TileSize::Large));
        assert!(!stds.contains_key(100, 100, ZoomLevel::L10, TileSize::Normal));
        assert!(stds.contains_key(100, 50, ZoomLevel::L10, TileSize::Normal));
        stds.insert(
            50,
            100,
            ZoomLevel::L18,
            TileSize::Large,
            "filename".into(),
            DownloadStatus::Downloaded,
        );
        assert!(!stds.contains_key(50, 100, ZoomLevel::L1, TileSize::Large));
        assert!(!stds.contains_key(50, 100, ZoomLevel::L18, TileSize::Normal));
        assert!(!stds.contains_key(100, 50, ZoomLevel::L18, TileSize::Large));
        assert!(stds.contains_key(50, 100, ZoomLevel::L18, TileSize::Large));
    }

    #[test]
    fn test_pixel_to_world_coords() {
        let tile_size = TileSize::Normal;
        let zoom_level = ZoomLevel::L18;
        // Bevy start (0,0) at the bottom left, so this is close to the bottom left on a map rendered in bevy.
        let world_coords = world_pixel_to_world_coords(42_125.0, 101_661.0, tile_size, zoom_level);
        let rounded = LatitudeLongitudeCoordinates {
            latitude: format!("{:.5}", world_coords.latitude).parse().unwrap(),
            longitude: format!("{:.5}", world_coords.longitude).parse().unwrap(),
        };
        // We expect world coordinates roughly in the bottom left on a world map.
        let check = LatitudeLongitudeCoordinates {
            latitude: -85.00386,
            longitude: -179.77402,
        };
        assert_eq!(rounded, check);
        let pixel = world_coords_to_world_pixel(&world_coords, tile_size, zoom_level);
        let rounded = (pixel.0.round(), pixel.1.round());
        assert_eq!(rounded, (42_125.0, 101_661.0));
    }

    #[test]
    fn test_world_coords_to_pixel() {
        let world_coords = LatitudeLongitudeCoordinates {
            latitude: 45.41098,
            longitude: -75.69854,
        };
        let tile_size = TileSize::Normal;
        let zoom_level = ZoomLevel::L18;
        let pixel = world_coords_to_world_pixel(&world_coords, tile_size, zoom_level);
        assert_eq!((pixel.0 as u32, pixel.1 as u32), (19_443_201, 43_076_862));
        let world_coords2 = world_pixel_to_world_coords(pixel.0, pixel.1, tile_size, zoom_level);
        let rounded = LatitudeLongitudeCoordinates {
            latitude: format!("{:.5}", world_coords2.latitude).parse().unwrap(),
            longitude: format!("{:.5}", world_coords2.longitude).parse().unwrap(),
        };
        assert_eq!(world_coords, rounded);
    }
}
