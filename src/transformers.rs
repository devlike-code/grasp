#[allow(dead_code)]
pub mod finite_state;
#[allow(dead_code)]
pub mod generate_enum;
#[allow(dead_code)]
pub mod pattern_match;
#[allow(dead_code)]
pub mod procedures;
#[allow(dead_code)]
pub mod select;

use std::sync::Arc;

pub use generate_enum::*;

pub use finite_state::*;
use mosaic::{
    internals::{Mosaic, MosaicIO},
    iterators::component_selectors::ComponentSelectors,
};
pub use pattern_match::*;
pub use procedures::*;

pub use select::*;

pub trait TransformerUtilities {
    fn is_transformer_pending(&self) -> bool;
}

impl TransformerUtilities for Arc<Mosaic> {
    fn is_transformer_pending(&self) -> bool {
        self.get_all()
            .include_component("WindowTransformerRequest")
            .len()
            > 0
    }
}
