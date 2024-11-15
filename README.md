# Bevy Slippy Tiles

[![Bevy Slippy Tiles](https://github.com/edouardpoitras/bevy_slippy_tiles/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/edouardpoitras/bevy_slippy_tiles/actions/workflows/rust.yml)
[![Latest version](https://img.shields.io/crates/v/bevy_slippy_tiles.svg)](https://crates.io/crates/bevy_slippy_tiles)
[![Documentation](https://docs.rs/bevy_slippy_tiles/badge.svg)](https://docs.rs/bevy_slippy_tiles)
![MIT](https://img.shields.io/badge/license-MIT-blue.svg)
![Apache](https://img.shields.io/badge/license-Apache-blue.svg)

A helper bevy plugin to handle downloading OpenStreetMap-compliant [slippy tiles](https://wiki.openstreetmap.org/wiki/Slippy_map_tilenames). Configurable concurrency, retries, and rate limit settings.

[`DownloadSlippyTilesEvent`] can be fired to request one or more slippy tile downloads.

[`SlippyTileDownloadedEvent`] is fired when a requested slippy tile has been retrieved successfully. The file path is stored in the event and can be used with the asset loader.

## Example

Here's a snippet of the example in this crate. This app will load a slippy tile and it's surrounding 8 tiles at the latitude and longitude specified.

Run with: `cargo run --example simple`


https://user-images.githubusercontent.com/14075649/214139995-c69fc4c7-634e-487a-af0d-a8ac42b6851f.mp4


```rust,ignore

// ...

const LATITUDE: f64 = 45.4111;
const LONGITUDE: f64 = -75.6980;

fn main() {
    App::new()
        // Our slippy tiles settings and plugin
        .insert_resource(SlippyTilesSettings {
            endpoint: "https://tile.openstreetmap.org", // Tile server endpoint.
            tiles_directory: "tiles/", // assets/ folder storing the slippy tile downloads.
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugins(SlippyTilesPlugin);

        // ...

}

// ...

fn request_slippy_tiles(mut download_slippy_tile_events: EventWriter<DownloadSlippyTilesEvent>) {

    // ...

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
    for slippy_tile_downloaded_event in slippy_tile_downloaded_events.read() {
        let zoom_level = slippy_tile_downloaded_event.zoom_level;

        // ..

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
```
## Bevy Compatibility

|bevy|bevy_slippy_tiles|
|---|---|
|0.14|0.7|
|0.13|0.5|
|0.12|0.4|
|0.11|0.3|
|0.10|0.2|
|0.9|0.1.3|
