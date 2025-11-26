# Bevy Slippy Tiles

[![Bevy Slippy Tiles](https://github.com/edouardpoitras/bevy_slippy_tiles/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/edouardpoitras/bevy_slippy_tiles/actions/workflows/rust.yml)
[![Latest version](https://img.shields.io/crates/v/bevy_slippy_tiles.svg)](https://crates.io/crates/bevy_slippy_tiles)
[![Documentation](https://docs.rs/bevy_slippy_tiles/badge.svg)](https://docs.rs/bevy_slippy_tiles)
![MIT](https://img.shields.io/badge/license-MIT-blue.svg)
![Apache](https://img.shields.io/badge/license-Apache-blue.svg)

A helper bevy plugin to handle downloading and displaying OpenStreetMap-compliant [slippy tiles](https://wiki.openstreetmap.org/wiki/Slippy_map_tilenames).

[`DownloadSlippyTilesMessage`] can be fired to request one or more slippy tile downloads.

[`SlippyTileDownloadedMessage`] is fired when a requested slippy tile has been retrieved successfully. The file path is stored in the message and can be used with the asset loader.

## Features

- Automatic tile rendering with configurable reference point
- Control over tile Z-layer for proper rendering order
- Optional transform offset for precise positioning
- Toggle automatic rendering for manual control
- Configurable download settings (concurrency, retries, rate limits)

## Example

Here's a snippet showing how to download and display map tiles at a specific location. This app will load a slippy tile and its surrounding 24 tiles at the specified latitude and longitude.

Run with: `cargo run --example simple`

```rust,no_run
use bevy::prelude::*;
use bevy_slippy_tiles::*;

const LATITUDE: f64 = 45.4111;
const LONGITUDE: f64 = -75.6980;

fn main() {
    App::new()
        // Configure settings with defaults
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

fn request_slippy_tiles(mut commands: Commands, mut download_slippy_tile_messages: MessageWriter<DownloadSlippyTilesMessage>) {
    commands.spawn(Camera2d::default());
    let slippy_tile_message = DownloadSlippyTilesMessage {
        tile_size: TileSize::Normal,    // Size of tiles - Normal = 256px, Large = 512px
        zoom_level: ZoomLevel::L18,     // Map zoom level (L0 = entire world, L19 = closest)
        coordinates: Coordinates::from_latitude_longitude(LATITUDE, LONGITUDE),
        radius: Radius(2),              // Request surrounding tiles (2 = 25 tiles total)
        use_cache: true,                // Use cached tiles if available
    };
    download_slippy_tile_messages.write(slippy_tile_message);
}
```

## Configuration

### SlippyTilesSettings

The plugin uses reasonable defaults but can be configured:
- `endpoint`: The tile server endpoint
- `tiles_directory`: The tile cache directory (where tiles will end up after being downloaded)
- `max_concurrent_downloads`: Maximum number of concurrent tile downloads
- `max_retries`: Maximum number of times a tile download will be retried upon failure
- `rate_limit_requests`: Maximum number of tile download requests within the rate limit window
- `rate_limit_window`: The duration of the rate limit window
- `reference_latitude`/`reference_longitude`: The geographic point that should appear at Transform(0,0,0) (or at transform_offset if specified)
- `transform_offset`: Optional Transform to offset where the reference point appears
- `z_layer`: Z coordinate for rendered tiles, useful for layering with other sprites
- `auto_render`: Toggle automatic tile rendering (disable for manual control)

```rust,no_run
# use bevy::prelude::Transform;
# use std::time::Duration;
# use bevy_slippy_tiles::SlippyTilesSettings;
SlippyTilesSettings {
    endpoint: "https://tile.openstreetmap.org".into(), // Tile server endpoint
    tiles_directory: "tiles/".into(), // Cache directory
    max_concurrent_downloads: 4, // Concurrent downloads
    max_retries: 3, // Download retry attempts
    rate_limit_requests: 10, // Rate limit requests
    rate_limit_window: Duration::from_secs(1), // Rate limit window
    reference_latitude: 45.4111, // Reference latitude
    reference_longitude: -75.6980, // Reference longitude
    transform_offset: Some( // Optional offset from 0,0 (default: None)
        Transform::from_xyz(100.0, 100.0, 0.0)
    ),
    z_layer: 1.0, // Z coordinate for tiles (default: 0.0)
    auto_render: true, // Enable automatic rendering (default: true)
}
# ;
```

### Cargo Features

This crate provides optional Cargo features for customization:

- **`display`** (enabled by default): Enables automatic Slippy tile rendering

To disable this feature:

```sh
cargo build --no-default-features
```

## Bevy Compatibility

|bevy|bevy_slippy_tiles|
|---|---|
|0.17|0.10|
|0.16|0.9|
|0.15|0.8|
|0.14|0.7|
|0.13|0.5|
|0.12|0.4|
|0.11|0.3|
|0.10|0.2|
|0.9|0.1.3|
