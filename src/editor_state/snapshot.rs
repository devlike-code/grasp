use std::sync::Arc;

use layout::{
    backends::svg::SVGWriter,
    core::utils::save_to_file,
    gv::{self, GraphBuilder},
    topo::layout::VisualGraph,
};
use mosaic::internals::Mosaic;

use super::management::GraspEditorState;

impl GraspEditorState {
    pub fn snapshot_all(&self, name: &str) {
        self.snapshot(format!("{}_EDITOR", name).as_str(), &self.editor_mosaic);

        for window in &self.window_list.windows {
            self.snapshot(
                format!("{}_WINDOW_{}", name, window.window_tile.id).as_str(),
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

    pub fn snapshot(&self, name: &str, mosaic: &Arc<Mosaic>) {
        let content = mosaic.dot(name);
        let mut parser = gv::DotParser::new(&content);
        match parser.process() {
            Ok(ast) => {
                println!("{:?}", ast);
                let mut gb = GraphBuilder::new();
                gb.visit_graph(&ast);
                let mut vg = gb.get();
                Self::generate_svg(&mut vg, name);
            }
            Err(err) => panic!("{:?}", err),
        }

        // let content = mosaic.dot(name);
        // open::that(format!(
        //     "https://dreampuf.github.io/GraphvizOnline/#{}",
        //     urlencoding::encode(content.as_str())
        // ))
        // .unwrap();
    }
}
