use std::sync::Arc;

use layout::{
    backends::svg::SVGWriter,
    core::utils::save_to_file,
    gv::{self, GraphBuilder},
    topo::layout::VisualGraph,
};
use mosaic::internals::Mosaic;

use super::{
    foundation::GraspEditorState,
    network::{Networked, DOTS},
};

impl GraspEditorState {
    pub fn snapshot_all(&self, name: &str) {
        self.open_snapshot(
            format!("{}_COMPS", name).as_str(),
            &self.component_mosaic,
            &self.component_mosaic,
        );

        self.open_snapshot(
            format!("{}_TRANSFORM", name).as_str(),
            &self.transformer_mosaic,
            &self.transformer_mosaic,
        );

        self.open_snapshot(
            format!("{}_EDITOR", name).as_str(),
            self,
            &self.editor_mosaic,
        );

        for window in &self.window_list.windows {
            self.open_snapshot(
                format!("{}_WINDOW_{}", name, window.window_tile.id).as_str(),
                &window.document_mosaic,
                &window.document_mosaic,
            );
        }
    }

    pub fn update_snapshot_all(&self, name: &str) {
        self.snapshot(
            format!("{}_COMPS", name).as_str(),
            self,
            &self.component_mosaic,
        );

        self.snapshot(
            format!("{}_EDITOR", name).as_str(),
            self,
            &self.editor_mosaic,
        );

        for window in &self.window_list.windows {
            self.snapshot(
                format!("{}_WINDOW_{}", name, window.window_tile.id).as_str(),
                &window.document_mosaic,
                &window.document_mosaic,
            );
        }
    }

    fn generate_svg(graph: &mut VisualGraph, name: &str) {
        let mut svg = SVGWriter::new();
        graph.do_it(false, false, false, &mut svg);
        let content = svg.finalize();

        let output_path = format!(".//{}.svg", name);
        let res = save_to_file(output_path.as_str(), &content);
        if let Result::Err(err) = res {
            log::error!("Could not write the file {}", output_path.as_str());
            log::error!("Error {}", err);
        }
    }

    pub fn open_snapshot(&self, name: &str, networked: &dyn Networked, mosaic: &Arc<Mosaic>) {
        let _ = open::that(format!("http://localhost:9000/#{}", networked.get_id()));
        self.snapshot(name, networked, mosaic);
    }

    pub fn snapshot(&self, _name: &str, networked: &dyn Networked, _mosaic: &Arc<Mosaic>) {
        let content = networked.prepare_content();
        let mut lock = DOTS.lock().unwrap();
        lock.insert(networked.get_id(), content);
    }
}
