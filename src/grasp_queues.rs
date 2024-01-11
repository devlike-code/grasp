use grasp_proc_macros::GraspQueue;
use itertools::Itertools;
use mosaic::{
    capabilities::{
        process::ProcessCapability, ArchetypeSubject, CollageImportCapability, StringCapability,
    },
    internals::{void, MosaicIO, Tile, TileFieldEmptyQuery},
    iterators::{
        component_selectors::ComponentSelectors, tile_deletion::TileDeletion,
        tile_getters::TileGetters,
    },
};
use std::{fs, vec::IntoIter};

use crate::{
    core::{
        gui::windowing::gui_set_window_focus,
        has_mosaic::HasMosaic,
        queues::{self, dequeue, GraspQueue},
    },
    editor_state::foundation::GraspEditorState,
    utilities::{Label, Process},
};

#[derive(GraspQueue)]
pub struct NewWindowRequestQueue;

#[derive(GraspQueue)]
pub struct NamedFocusWindowRequestQueue;

#[derive(GraspQueue)]
pub struct CloseWindowRequestQueue;

#[derive(GraspQueue)]
pub struct QuadtreeUpdateRequestQueue;

#[derive(GraspQueue)]
pub struct WindowMessageInboxQueue(Tile);

#[derive(GraspQueue)]
pub struct WindowTransformerQueue;

impl GraspEditorState {
    fn iter_all_windows(&self) -> IntoIter<Tile> {
        //each window tile has arrow "DirectWindowRequest" pointing to "Queue" tile that has descriptor "EditorWindowQueue" attached, and descriptors
        self.editor_state_tile
            .iter()
            .get_arrows_from()
            .include_component("DirectWindowRequest")
            .get_targets()
    }

    //processing all queues on Editor level
    pub fn process_requests(&mut self) {
        self.process_named_focus_window_queue();
        self.process_new_window_queue();
        self.process_quadtree_queue();
        self.process_close_window_queue();
        self.process_window_transformer_queue();
    }

    fn process_named_focus_window_queue(&mut self) {
        while let Some(request) = queues::dequeue(NamedFocusWindowRequestQueue, &self.editor_mosaic)
        {
            let data = request.get("self").as_s32();

            if let Some(pos) = self
                .window_list
                .windows
                .iter()
                .position(|w| w.name == data.to_string())
            {
                let window = self.window_list.windows.remove(pos).unwrap();
                self.window_list.windows.push_front(window);
                gui_set_window_focus(&data.to_string());
            }
        }
    }

    fn process_new_window_queue(&mut self) {
        while let Some(request) = queues::dequeue(NewWindowRequestQueue, &self.editor_mosaic) {
            // TODO: reconnect collage, but with reconstruction into other mosaic
            if let Some(_collage) = request.to_collage() {
                self.new_window(None);
                request.iter().delete();
            }
        }
    }

    fn process_close_window_queue(&mut self) {
        while let Some(request) = queues::dequeue(CloseWindowRequestQueue, &self.editor_mosaic) {
            self.close_window(self.window_list.get_focused().unwrap().window_tile.clone());
            request.iter().delete();
        }
    }
    fn process_quadtree_queue(&mut self) {
        while let Some(request) = dequeue(QuadtreeUpdateRequestQueue, &self.editor_mosaic) {
            let all_window_queues = self.iter_all_windows();
            for window_queue in all_window_queues {
                queues::enqueue_direct(
                    window_queue,
                    self.editor_mosaic
                        .new_object("QuadtreeUpdateRequest", void()),
                )
            }
            request.iter().delete();
        }
    }
    fn process_window_transformer_queue(&mut self) {
        while let Some(request) = dequeue(WindowTransformerQueue, &self.editor_mosaic) {
            let transformer_id = request.get("transform").as_u64() as usize;
            let window_index = request.get("window_index").as_u64() as usize;
            if let Some(window) = self.window_list.windows.get(window_index) {
                if let Some(transformer_template) = window.transformer_mosaic.get(transformer_id) {
                    //let transformer_name = Label(&transformer).query();
                    let fn_name = Process(&transformer_template).query();

                    if let Some(func) = self.transformer_functions.get(&fn_name) {
                        let input_templates =
                            transformer_template.iter().get_arrows_into().get_sources();

                        let params = input_templates
                            .clone()
                            .map(|i| Label(&i).query())
                            .collect_vec();
                        let params2 = params.iter().map(|s| s.as_str()).collect_vec();
                        let str_slice: &[&str] = &params2;

                        let proc_tile = window
                            .document_mosaic
                            .create_process(&fn_name, &str_slice)
                            .unwrap();

                        for input_template in input_templates.collect_vec() {
                            let input_name = Label(&input_template).query();

                            let input_instance = window.editor_data.selected.first().unwrap();

                            for validation in input_template.iter().get_descriptors() {
                                match validation.component.to_string().as_str() {
                                    "NoArrowsInto" => {
                                        if !input_instance
                                            .iter()
                                            .get_arrows_into()
                                            .collect_vec()
                                            .is_empty()
                                        {
                                            panic!("'NoArrowsInto' Input validation failed!")
                                        }
                                    }
                                    "HasArrowsInto" => {
                                        if input_instance
                                            .iter()
                                            .get_arrows_into()
                                            .collect_vec()
                                            .is_empty()
                                        {
                                            panic!("'HasArrowsInto' Input validation failed!")
                                        }
                                    }
                                    "HasComponent" => {
                                        let necessary_comp_name =
                                            validation.get("self").as_s32().to_string();

                                        if input_instance
                                            .get_component(&necessary_comp_name)
                                            .is_none()
                                        {
                                            panic!("'HasArrowsInto' Input validation failed!")
                                        }
                                    }
                                    "HasArrowsFrom" => {
                                        if input_instance
                                            .iter()
                                            .get_arrows_from()
                                            .collect_vec()
                                            .is_empty()
                                        {
                                            panic!("'NoArrowsFrom' Input validation failed!")
                                        }
                                    }
                                    "NoArrowsFrom" => {
                                        if !input_instance
                                            .iter()
                                            .get_arrows_from()
                                            .collect_vec()
                                            .is_empty()
                                        {
                                            panic!("'NoArrowsFrom' Input validation failed!")
                                        }
                                    }
                                    _ => {
                                        println!("'UNKOWN' Input validation failed!")
                                    }
                                }
                            }

                            window
                                .document_mosaic
                                .pass_process_parameter(&proc_tile, &input_name, input_instance)
                                .unwrap();

                            let result = (func)(&proc_tile);
                            fs::write(
                                "MyEnum.cs",
                                format!(
                                    "{}",
                                    window.document_mosaic.get_string_value(&result).unwrap()
                                ),
                            )
                            .unwrap();
                        }
                    }
                }
            }

            request.iter().delete();
        }
    }
}
