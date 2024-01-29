use crate::{
    editor_state::{
        foundation::GraspEditorState,
        selection::{pick_n_renderer, selection_renderer},
        view::{
            color_property_renderer, selected_property_renderer, two_float_property_xy_renderer,
        },
    },
    transformers::procedure_renderer,
};

pub fn setup_component_renderers(instance: &mut GraspEditorState) {
    for i in 1..=5 {
        instance
            .hidden_property_renderers
            .insert(format!("Pick{}", i));
    }

    instance
        .hidden_property_renderers
        .insert("Color".to_string());

    instance
        .hidden_property_renderers
        .insert("ProcedureResult".to_string());

    instance
        .hidden_property_renderers
        .insert("ProcedureArgument".to_string());

    instance
        .hidden_property_renderers
        .insert("Selected".to_string());

    instance
        .hidden_property_renderers
        .insert("SelectionOwner".to_string());

    instance
        .hidden_property_renderers
        .insert("List".to_string());

    instance
        .hidden_property_renderers
        .insert("Pair".to_string());

    instance
        .component_entity_renderers
        .insert("SelectionOwner".into(), Box::new(selection_renderer));

    for i in 1..=5 {
        instance
            .component_entity_renderers
            .insert(format!("Pick{}", i), Box::new(pick_n_renderer(i)));
    }

    instance
        .component_property_renderers
        .insert("Procedure".into(), Box::new(procedure_renderer));

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
