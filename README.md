# Bevy Slippy Tiles

A helper bevy plugin to handle downloading OpenStreetMap-compliant [slippy tiles](https://wiki.openstreetmap.org/wiki/Slippy_map_tilenames).

[`DownloadSlippyTilesEvent`] can be fired to request one or more slippy tile downloads.

[`SlippyTileDownloadedEvent`] is fired when a requested slippy tile has been retrieved successfully. The file path is stored in the event and can be used with the asset loader.

## Example

Here's a snippet of the example in this crate. This app will load a slippy tile and it's surrounding 8 tiles at the latitude and longitude specified.

Run with: `cargo run --example simple`

https://raw.githubusercontent.com/edouardpoitras/bevy_slippy_tiles/main/simple-example.mp4

```rust
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
    for slippy_tile_downloaded_event in slippy_tile_downloaded_events.iter() {

        // ...

        commands.spawn(SpriteBundle {
            texture: asset_server.load(slippy_tile_downloaded_event.path.clone()),
            transform: Transform::from_xyz(transform_x, transform_y, 0.0),
            ..Default::default()
        });
    }
}
```
