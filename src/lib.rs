#![doc = include_str!("../README.md")]

mod constants;
mod coordinates;
#[cfg(feature = "display")]
mod display;
mod download;
mod settings;
mod systems;
mod types;

pub use constants::*;
pub use coordinates::*;
#[cfg(feature = "display")]
pub use display::*;
pub use download::*;
pub use settings::*;
pub use types::*;

use bevy::prelude::{App, Plugin, Startup, Update};

pub struct SlippyTilesPlugin;

impl Plugin for SlippyTilesPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SlippyTileDownloadStatus::new())
            .insert_resource(SlippyTileDownloadTasks::new())
            .insert_resource(systems::DownloadRateLimiter::default())
            .add_message::<DownloadSlippyTilesMessage>()
            .add_message::<SlippyTileDownloadedMessage>()
            .add_systems(Startup, systems::initialize_semaphore)
            .add_systems(Update, systems::download_slippy_tiles)
            .add_systems(Update, systems::download_slippy_tiles_completed);

        #[cfg(feature = "display")]
        app.add_systems(Update, display::display_tiles);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_approx_eq(a: f64, b: f64, epsilon: f64) {
        assert!(
            (a - b).abs() < epsilon,
            "Expected {} to be approximately equal to {}",
            a,
            b
        );
    }

    fn assert_coords_approx_eq(a: &LatitudeLongitudeCoordinates, b: &LatitudeLongitudeCoordinates) {
        assert_approx_eq(a.latitude, b.latitude, 1e-14);
        assert_approx_eq(a.longitude, b.longitude, 1e-14);
    }

    #[test]
    fn test_zoom_level_try_from() {
        assert_eq!(ZoomLevel::try_from(0), Ok(ZoomLevel::L0));
        assert_eq!(ZoomLevel::try_from(25), Ok(ZoomLevel::L25));
        assert_eq!(ZoomLevel::try_from(26), Err(()));
    }

    #[test]
    fn test_tile_size_new() {
        assert_eq!(TileSize::new(256), TileSize::Normal);
        assert_eq!(TileSize::new(512), TileSize::Large);
        assert_eq!(TileSize::new(768), TileSize::VeryLarge);
        assert_eq!(TileSize::new(1024), TileSize::Normal);
    }

    #[test]
    fn test_slippy_tile_settings() {
        let sts = SlippyTilesSettings {
            endpoint: "endpoint".into(),
            tiles_directory: "tiles_directory".into(),
            ..Default::default()
        };
        assert_eq!(sts.endpoint, "endpoint");
        assert_eq!(
            sts.tiles_directory,
            std::path::PathBuf::from("tiles_directory")
        );
        assert_eq!(sts.get_tiles_directory_string(), "tiles_directory");
        assert!(std::path::Path::try_exists("assets/tiles_directory".as_ref()).is_ok());
    }

    #[test]
    fn test_slippy_tile_coordinates_l0() {
        assert_eq!(
            SlippyTileCoordinates::from_latitude_longitude(0.0, 0.0, ZoomLevel::L0),
            SlippyTileCoordinates { x: 0, y: 0 }
        );
        assert_eq!(
            SlippyTileCoordinates::from_latitude_longitude(-89.0, -179.0, ZoomLevel::L0),
            SlippyTileCoordinates { x: 0, y: 1 }
        );
        assert_eq!(
            SlippyTileCoordinates::from_latitude_longitude(89.0, 179.0, ZoomLevel::L0),
            SlippyTileCoordinates { x: 0, y: 0 }
        );
        assert_coords_approx_eq(
            &SlippyTileCoordinates::to_latitude_longitude(
                &SlippyTileCoordinates { x: 1, y: 1 },
                ZoomLevel::L0,
            ),
            &LatitudeLongitudeCoordinates {
                latitude: -85.0511287798066,
                longitude: 180.0,
            },
        );
    }

    #[test]
    fn test_slippy_tile_coordinates_l1() {
        assert_coords_approx_eq(
            &SlippyTileCoordinates::to_latitude_longitude(
                &SlippyTileCoordinates { x: 1, y: 0 },
                ZoomLevel::L1,
            ),
            &LatitudeLongitudeCoordinates {
                latitude: 85.0511287798066,
                longitude: 0.0,
            },
        );
        assert_eq!(
            SlippyTileCoordinates::from_latitude_longitude(0.0, 175.0, ZoomLevel::L1),
            SlippyTileCoordinates { x: 1, y: 1 }
        );
        assert_coords_approx_eq(
            &SlippyTileCoordinates::to_latitude_longitude(
                &SlippyTileCoordinates { x: 1, y: 1 },
                ZoomLevel::L1,
            ),
            &LatitudeLongitudeCoordinates {
                latitude: 0.0,
                longitude: 0.0,
            },
        );
    }

    #[test]
    fn test_slippy_tile_coordinates() {
        let coords = SlippyTileCoordinates::from_latitude_longitude(0.0, 0.0, ZoomLevel::L10);
        assert_eq!(coords, SlippyTileCoordinates { x: 512, y: 512 });

        let result = SlippyTileCoordinates::to_latitude_longitude(
            &SlippyTileCoordinates { x: 512, y: 512 },
            ZoomLevel::L10,
        );
        assert_coords_approx_eq(
            &result,
            &LatitudeLongitudeCoordinates {
                latitude: 0.0,
                longitude: 0.0,
            },
        );

        let coords = SlippyTileCoordinates::from_latitude_longitude(
            48.81590713080016,
            2.2686767578125,
            ZoomLevel::L17,
        );
        assert_eq!(coords, SlippyTileCoordinates { x: 66362, y: 45115 });

        let result = SlippyTileCoordinates::to_latitude_longitude(
            &SlippyTileCoordinates { x: 66362, y: 45115 },
            ZoomLevel::L17,
        );
        assert_coords_approx_eq(
            &result,
            &LatitudeLongitudeCoordinates {
                latitude: 48.81590713080016,
                longitude: 2.2686767578125,
            },
        );

        let coords = SlippyTileCoordinates::from_latitude_longitude(
            0.004806518549043551,
            0.004119873046875,
            ZoomLevel::L19,
        );
        assert_eq!(
            coords,
            SlippyTileCoordinates {
                x: 262150,
                y: 262137
            }
        );

        let result = SlippyTileCoordinates::to_latitude_longitude(
            &SlippyTileCoordinates {
                x: 262150,
                y: 262137,
            },
            ZoomLevel::L19,
        );
        assert_coords_approx_eq(
            &result,
            &LatitudeLongitudeCoordinates {
                latitude: 0.004806518549043551,
                longitude: 0.004119873046875,
            },
        );

        let coords = SlippyTileCoordinates::from_latitude_longitude(
            26.850416392948524,
            72.57980346679688,
            ZoomLevel::L19,
        );
        assert_eq!(
            coords,
            SlippyTileCoordinates {
                x: 367846,
                y: 221525
            }
        );

        let result = SlippyTileCoordinates::to_latitude_longitude(
            &SlippyTileCoordinates {
                x: 367846,
                y: 221525,
            },
            ZoomLevel::L19,
        );
        assert_coords_approx_eq(
            &result,
            &LatitudeLongitudeCoordinates {
                latitude: 26.850416392948524,
                longitude: 72.57980346679688,
            },
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
        // We expect world coordinates roughly in the bottom left on a world map.
        assert_approx_eq(world_coords.latitude, -85.00385927087456, 1e-14);
        assert_approx_eq(world_coords.longitude, -179.7740238904953, 1e-14);
        let pixel = world_coords_to_world_pixel(&world_coords, tile_size, zoom_level);
        assert_approx_eq(pixel.0, 42_125.0, 1e-5);
        assert_approx_eq(pixel.1, 101_661.0, 1e-5);
    }

    #[test]
    fn test_world_coords_to_pixel() {
        let tile_size = TileSize::Normal;
        let zoom_level = ZoomLevel::L18;
        let world_coords = LatitudeLongitudeCoordinates {
            latitude: 45.41097678404845,
            longitude: -75.69854199886322,
        };
        let pixel = world_coords_to_world_pixel(&world_coords, tile_size, zoom_level);
        assert_approx_eq(pixel.0, 19_443_201.0, 1e-14);
        assert_approx_eq(pixel.1, 43_076_862.0, 1e-14);
        let world_coords2 = world_pixel_to_world_coords(pixel.0, pixel.1, tile_size, zoom_level);
        assert_approx_eq(world_coords.latitude, world_coords2.latitude, 1e-14);
        assert_approx_eq(world_coords.longitude, world_coords2.longitude, 1e-14);
    }
}
