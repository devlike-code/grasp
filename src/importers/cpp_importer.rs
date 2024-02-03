use std::{path::PathBuf, str::FromStr};

use crate::{core::math::Vec2, editor_state::windows::GraspEditorWindow};
use mosaic::{
    capabilities::ArchetypeSubject,
    internals::{Tile, TileFieldSetter, S32},
};
use tree_sitter::{Node, Parser};

fn import_node(window: &mut GraspEditorWindow, parent: Tile, node: Node<'_>, pos: &mut Vec2) {
    let original_pos = *pos;
    let children_count = node.child_count();

    let dx = children_count as f32 * 0.5 * 200.0;
    let mut new_pos = Vec2::new(pos.x - dx, pos.y + 100.0);

    let mut walk = node.walk();
    let mut walking = walk.goto_first_child();

    while walking {
        let child = walk.node();
        let child_node = window.create_new_object(new_pos);
        child_node
            .get_component("Label")
            .unwrap()
            .set("self", S32::from_str(child.kind()).unwrap());
        window.create_new_arrow(
            &parent.clone(),
            &child_node,
            original_pos.lerp(new_pos, 0.5),
        );

        import_node(window, child_node.clone(), child, &mut new_pos.clone());

        walking = walk.goto_next_sibling();
        new_pos.x += dx;
    }
}

pub fn cpp_importer(window: &mut GraspEditorWindow, content: String, path: PathBuf) {
    let mut parser = Parser::new();
    parser
        .set_language(tree_sitter_cpp::language())
        .expect("Error loading cpp grammar");

    let mut pos = Vec2::new(500.0, 100.0);
    if let Some(parsed) = parser.parse(content, None) {
        let root = parsed.root_node();

        let node = window.create_new_object(pos);
        node.get_component("Label")
            .unwrap()
            .set("self", S32::from_str(root.kind()).unwrap());

        import_node(window, node, root, &mut pos);
    }
}
