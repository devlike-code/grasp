use std::sync::Arc;

use mosaic::internals::{Mosaic, Tile};

pub trait HasMosaic {
    fn get_mosaic(&self) -> Arc<Mosaic>;
}

impl HasMosaic for Tile {
    fn get_mosaic(&self) -> Arc<Mosaic> {
        Arc::clone(&self.mosaic)
    }
}

impl HasMosaic for Arc<Mosaic> {
    fn get_mosaic(&self) -> Arc<Mosaic> {
        Arc::clone(self)
    }
}
