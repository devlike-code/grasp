use std::sync::Arc;

use mosaic::{
    capabilities::ArchetypeSubject,
    internals::{par, void, Mosaic, MosaicIO, Tile, TileFieldEmptyQuery, Value},
};

use crate::{
    core::{has_mosaic::HasMosaic, structures::grasp_queues},
    grasp_queues::{NamedFocusWindowRequestQueue, QuadtreeUpdateRequestQueue},
};

use super::foundation::GraspEditorState;

pub struct DisplayName<'a>(pub &'a Tile);
impl<'a> TileFieldEmptyQuery for DisplayName<'a> {
    type Output = Option<String>;
    fn query(&self) -> Self::Output {
        if let Some(pos_component) = self.0.get_component("DisplayName") {
            if let Value::S32(s) = pos_component.get("self") {
                return Some(s.to_string());
            }
        }

        None
    }
}

pub trait RequireWindowFocus: HasMosaic {
    fn require_window_focus(&self, window: Tile) {
        grasp_queues::enqueue_direct(
            window,
            self.get_mosaic().new_object("FocusWindowRequest", void()),
        );
    }

    fn require_named_window_focus(&self, name: &str) {
        grasp_queues::enqueue(
            NamedFocusWindowRequestQueue,
            self.get_mosaic()
                .new_object("NamedFocusWindowRequest", par(name)),
        );
    }
}

impl HasMosaic for GraspEditorState {
    fn get_mosaic(&self) -> Arc<Mosaic> {
        Arc::clone(&self.editor_mosaic)
    }
}

impl RequireWindowFocus for GraspEditorState {}

pub trait QuadtreeUpdateCapability {
    fn request_quadtree_update(&self);
}

impl QuadtreeUpdateCapability for Arc<Mosaic> {
    fn request_quadtree_update(&self) {
        grasp_queues::enqueue(
            QuadtreeUpdateRequestQueue,
            self.new_object("QuadtreeUpdateRequest", void()),
        );
    }
}
