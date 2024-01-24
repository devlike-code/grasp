use crate::editor_state::{
    foundation::GraspEditorState,
    selection::selection_renderer,
    view::{color_property_renderer, selected_property_renderer, two_float_property_xy_renderer},
};

pub fn setup_component_renderers(instance: &mut GraspEditorState) {
    instance
        .component_entity_renderers
        .insert("SelectionOwner".into(), Box::new(selection_renderer));

    instance
        .component_property_renderers
        .insert("Position".into(), Box::new(two_float_property_xy_renderer));

    instance
        .component_property_renderers
        .insert("Offset".into(), Box::new(two_float_property_xy_renderer));

    instance
        .component_property_renderers
        .insert("Color".into(), Box::new(color_property_renderer));

    instance
        .component_property_renderers
        .insert("Selected".into(), Box::new(selected_property_renderer));
}
