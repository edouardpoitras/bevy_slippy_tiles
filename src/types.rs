macro_rules! generate_zoom_level {
    { $( $name:ident => $val:literal, )+ } => {
        /// The zoom level used when fetching tiles (0 <= zoom <= 25)
        #[derive(Eq, Hash, PartialEq, Clone, Copy, Debug)]
        pub enum ZoomLevel {
            $( $name = $val, )+
        }

        impl ZoomLevel {
            pub fn to_u8(&self) -> u8 {
                *self as u8
            }
        }

        impl TryFrom<u8> for ZoomLevel {
            type Error = ();

            fn try_from(v: u8) -> Result<Self, Self::Error> {
                match v {
                    $( $val => Ok(Self::$name), )+
                    _ => Err(()),
                }
            }
        }
    };
}

generate_zoom_level! {
    L0 => 0,
    L1 => 1,
    L2 => 2,
    L3 => 3,
    L4 => 4,
    L5 => 5,
    L6 => 6,
    L7 => 7,
    L8 => 8,
    L9 => 9,
    L10 => 10,
    L11 => 11,
    L12 => 12,
    L13 => 13,
    L14 => 14,
    L15 => 15,
    L16 => 16,
    L17 => 17,
    L18 => 18,
    L19 => 19,
    L20 => 20,
    L21 => 21,
    L22 => 22,
    L23 => 23,
    L24 => 24,
    L25 => 25,
}

/// The size of the tiles being requested - either 256px (Normal), 512px (Large), or 768px (VeryLarge).
/// Not every tile provider supports Large and VeryLarge.
#[derive(Eq, Hash, PartialEq, Clone, Copy, Debug)]
pub enum TileSize {
    Normal,
    Large,
    VeryLarge,
}

impl TileSize {
    /// Create a new TileSize type given a pixel count (512px = TileSize::Large, every other value is TileSize::Normal).
    pub fn new(tile_pixels: u32) -> TileSize {
        match tile_pixels {
            768 => TileSize::VeryLarge,
            512 => TileSize::Large,
            _ => TileSize::Normal,
        }
    }

    /// Returns the number of tile pixels given a TileSize variant.
    pub fn to_pixels(&self) -> u32 {
        match self {
            TileSize::Normal => 256,
            TileSize::Large => 512,
            TileSize::VeryLarge => 768,
        }
    }

    pub fn get_url_postfix(&self) -> String {
        match self {
            TileSize::Normal => "".into(),
            TileSize::Large => "@2x".into(),
            TileSize::VeryLarge => "@3x".into(),
        }
    }
}

/// Number of tiles away from the main tile that should be fetched. Effectively translates to layers of surrounding tiles. Will degrade performance exponentially.
///
/// Radius(0) = 1 tile (1x1), Radius(1) = 9 tiles (3x3), Radius(2) = 25 tiles (5x5), Radius(3) = 49 tiles (7x7), etc.
#[derive(Debug)]
pub struct Radius(pub u8);

pub enum UseCache {
    Yes,
    No,
}

impl UseCache {
    pub fn new(value: bool) -> UseCache {
        match value {
            true => UseCache::Yes,
            _ => UseCache::No,
        }
    }
}

pub enum AlreadyDownloaded {
    Yes,
    No,
}

impl AlreadyDownloaded {
    pub fn new(value: bool) -> AlreadyDownloaded {
        match value {
            true => AlreadyDownloaded::Yes,
            _ => AlreadyDownloaded::No,
        }
    }
}

pub enum FileExists {
    Yes,
    No,
}

impl FileExists {
    pub fn new(value: bool) -> FileExists {
        match value {
            true => FileExists::Yes,
            _ => FileExists::No,
        }
    }
}

#[derive(Debug)]
pub enum DownloadStatus {
    Downloading,
    Downloaded,
}
