use std::sync::Arc;

use mosaic::internals::{pars, ComponentValuesBuilderSetter, Mosaic, MosaicIO, Tile};

pub trait ErrorCapability {
    fn make_error(&self, message: &str, window: Option<Tile>, target: Option<Tile>);
}

impl ErrorCapability for Arc<Mosaic> {
    fn make_error(&self, message: &str, window: Option<Tile>, target: Option<Tile>) {
        self.new_object(
            "Error",
            pars()
                .set("message", message.to_string())
                .set("window", window.map(|t| t.id as u64).unwrap_or(0u64))
                .set("target", target.map(|t| t.id as u64).unwrap_or(0u64))
                .ok(),
        );
    }
}
