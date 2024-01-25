use grasp_proc_macros::GraspQueue;
use imgui::sys::igSetNextWindowPos;
use itertools::Itertools;
use mosaic::{
    capabilities::ArchetypeSubject,
    internals::{pars, void, ComponentValuesBuilderSetter, MosaicIO, Tile, TileFieldEmptyQuery},
    iterators::{
        component_selectors::ComponentSelectors, tile_deletion::TileDeletion,
        tile_getters::TileGetters,
    },
};
use std::vec::IntoIter;

use crate::{
    core::{
        gui::windowing::gui_set_window_focus,
        structures::grasp_queues::{self, dequeue, GraspQueue},
    },
    editor_state::{
        file_operations::SaveFileCapability,
        foundation::{GraspEditorState, TransformerState},
    },
    editor_state_machine::{EditorStateTrigger, StateMachine},
    GuiState,
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

#[derive(GraspQueue)]
pub struct WindowRenameRequestQueue;

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
    pub fn process_requests(&mut self, ui: &GuiState) {
        self.process_rename_window_queue();
        self.process_named_focus_window_queue();
        self.process_new_window_queue();
        self.process_quadtree_queue();
        self.process_close_window_queue(ui);
        self.process_window_transformer_queue(ui);
    }

    fn process_rename_window_queue(&mut self) {
        while let Some(request) =
            grasp_queues::dequeue(WindowRenameRequestQueue, &self.editor_mosaic)
        {
            let id = request.get("id").as_u64() as usize;
            let index = request.get("index").as_u64();
            let name = format!("[{}] {}", index, request.get("name").as_s32());

            if let Some(window) = self.window_list.get_by_id(id) {
                let old_name = window.name.clone();
                self.window_list.named_windows.iter_mut().for_each(|w| {
                    if *w == old_name {
                        *w = name.clone();
                    }
                });
            }

            if let Some(window) = self.window_list.get_by_id_mut(id) {
                window.name = name;
            }

            request.iter().delete();
        }
    }

    fn process_named_focus_window_queue(&mut self) {
        while let Some(request) =
            grasp_queues::dequeue(NamedFocusWindowRequestQueue, &self.editor_mosaic)
        {
            let data = request.get("self").as_str();

            if let Some(pos) = self.window_list.windows.iter().position(|w| w.name == data) {
                let window = self.window_list.windows.remove(pos).unwrap();
                self.window_list.windows.push_front(window);
                gui_set_window_focus(&data.to_string());
            }
        }
    }

    fn process_new_window_queue(&mut self) {
        while let Some(request) = grasp_queues::dequeue(NewWindowRequestQueue, &self.editor_mosaic)
        {
            self.new_window(None);
            request.iter().delete();
        }
    }

    fn process_close_window_queue(&mut self, ui: &GuiState) {
        ui.set_next_item_width(320.0);

        if let Some(_token) = ui
            .modal_popup_config("Unsaved changes")
            .save_settings(false)
            .always_auto_resize(true)
            .collapsible(false)
            .begin_popup()
        {
            if let Some(request) = self.pending_close_window_request.clone() {
                ui.text("Changes were made but unsaved.");
                if ui.button_with_size("Save", [100.0, 20.0]) {
                    self.save_file();
                    self.close_window(self.window_list.get_focused().unwrap().window_tile.clone());
                    request.iter().delete();
                    self.pending_close_window_request = None;
                    ui.close_current_popup();
                }
                ui.same_line();
                if ui.button_with_size("Discard", [100.0, 20.0]) {
                    self.close_window(self.window_list.get_focused().unwrap().window_tile.clone());
                    request.iter().delete();
                    self.pending_close_window_request = None;
                    ui.close_current_popup();
                }
                ui.same_line();
                if ui.button_with_size("Cancel", [100.0, 20.0]) {
                    request.iter().delete();
                    ui.close_current_popup();
                    self.pending_close_window_request = None;
                }
            }
        }

        while let Some(request) =
            grasp_queues::dequeue(CloseWindowRequestQueue, &self.editor_mosaic)
        {
            let window = self.window_list.get_focused().unwrap();
            if window.changed {
                self.pending_close_window_request = Some(request);
                ui.open_popup("Unsaved changes");
            } else {
                self.close_window(self.window_list.get_focused().unwrap().window_tile.clone());
                request.iter().delete();
            }
        }
    }

    fn process_quadtree_queue(&mut self) {
        while let Some(request) = dequeue(QuadtreeUpdateRequestQueue, &self.editor_mosaic) {
            let all_window_queues = self.iter_all_windows();
            for window_queue in all_window_queues {
                grasp_queues::enqueue_direct(
                    window_queue,
                    self.editor_mosaic
                        .new_object("QuadtreeUpdateRequest", void()),
                )
            }
            request.iter().delete();
        }
    }
}

pub enum ArrowMultiplicity {
    ExactlyOne,
    ZeroOrMore,
    OneOrMore,
}

pub struct Multiplicity<'a>(pub &'a Tile);
impl<'a> TileFieldEmptyQuery for Multiplicity<'a> {
    type Output = ArrowMultiplicity;
    fn query(&self) -> Self::Output {
        println!(
            "MULTIPLICITY ARCH: {:?}",
            self.0.iter().get_descriptors().collect_vec()
        );
        if self.0.get_component("OneOrMore").is_some() {
            ArrowMultiplicity::OneOrMore
        } else if self.0.get_component("ExactlyOne").is_some() {
            ArrowMultiplicity::ExactlyOne
        } else {
            ArrowMultiplicity::ZeroOrMore
        }
    }
}

impl GraspEditorState {
    #[allow(dead_code)]
    fn validate_against_template(
        &self,
        template: &Tile,
        instance: &Tile,
        fn_name: &str,
        window_index: u64,
        visited: &mut Vec<usize>,
    ) -> usize {
        println!("Validating {} against {}", instance.id, template.id);
        /* check object validations */
        let mut error_count = 0;

        for validation in template.iter().get_descriptors() {
            let maybe_error = match validation.component.to_string().as_str() {
                "NoArrowsInto" => {
                    println!("No arrows into");
                    if !instance.iter().get_arrows_into().collect_vec().is_empty() {
                        println!("\tarrows found");
                        Some(format!("This tile requires no that no arrows go into it for transformer {} to work.", fn_name))
                    } else {
                        None
                    }
                }
                "HasComponent" => {
                    let necessary_comp_name = validation.get("self").as_s32().to_string();
                    println!("Has component: {}", necessary_comp_name);
                    if instance.get_component(&necessary_comp_name).is_none() {
                        println!("\tno component found!");
                        Some(format!(
                            "Component '{}' required on this tile for transformer {} to work.",
                            necessary_comp_name, fn_name
                        ))
                    } else {
                        None
                    }
                }
                "NoArrowsFrom" => {
                    println!("No arrows from");
                    if !instance.iter().get_arrows_from().collect_vec().is_empty() {
                        println!("\tarrows found");
                        Some(format!("This tile requires no that no arrows go from it for transformer {} to work.", fn_name))
                    } else {
                        None
                    }
                }
                _ => None,
            };

            maybe_error.map(|err| {
                error_count += 1;
                self.editor_mosaic.new_object(
                    "Error",
                    pars()
                        .set("message", err)
                        .set("target", instance.id as u64)
                        .set("window", window_index)
                        .ok(),
                )
            });
        }

        visited.push(template.id);

        /* check arrows in template */
        for arrow in template.iter().get_arrows() {
            println!("Found arrow: {:?}", arrow);
            let is_outgoing = &arrow.source() == template;
            println!("\toutgoing?: {}", is_outgoing);
            let template_neighbor = if &arrow.source() == template {
                arrow.target()
            } else {
                arrow.source()
            };
            println!("\ttemplate neighbor: {:?}", template_neighbor);

            if visited.contains(&template_neighbor.id) {
                println!("\t\ttemplate neighbor checked already, discontinuing.");
                continue;
            }

            let instance_oriented_arrows = if is_outgoing {
                instance.iter().get_arrows_from()
            } else {
                instance.iter().get_arrows_into()
            };

            let is_correct = match Multiplicity(&arrow).query() {
                ArrowMultiplicity::ExactlyOne => instance_oriented_arrows.len() == 1,
                ArrowMultiplicity::OneOrMore => instance_oriented_arrows.len() >= 1,
                ArrowMultiplicity::ZeroOrMore => true,
            };
            println!(
                "\tinstance oriented arrows len = {:?}",
                instance_oriented_arrows.len()
            );

            println!("\tis multiplicity correct?: {}", is_correct);

            if !is_correct {
                self.editor_mosaic.new_object(
                    "Error",
                    pars()
                        .set(
                            "message",
                            format!(
                                "Multiplicity wrong for arrows going {} tile #{}",
                                if is_outgoing { "from" } else { "into" },
                                template.id
                            ),
                        )
                        .set("target", instance.id as u64)
                        .set("window", window_index)
                        .ok(),
                );
            } else {
                let instance_neighbors = if is_outgoing {
                    instance_oriented_arrows.get_targets()
                } else {
                    instance_oriented_arrows.get_sources()
                };
                /* iterate over instance neighbors */

                for instance_neighbor in instance_neighbors {
                    println!("Recurse for {}", instance_neighbor.id);
                    /* recursively call validation of instance neighbor against template neighbor */
                    error_count += self.validate_against_template(
                        &template_neighbor,
                        &instance_neighbor,
                        fn_name,
                        window_index,
                        visited,
                    );
                }
            }
        }

        error_count
    }

    fn process_window_transformer_queue(&mut self, ui: &GuiState) {
        while let Some(request) = dequeue(WindowTransformerQueue, &self.editor_mosaic) {
            let transformer = request.get("transform").as_s32().to_string();
            let window_index = request.get("window_index").as_u64() as usize;

            request.iter().delete();

            if let Some(window) = self
                .window_list
                .windows
                .iter()
                .find(|w| w.window_tile.id == window_index)
            {
                self.editor_mosaic
                    .get_all()
                    .include_component("Error")
                    .filter(|t| t.get("window").as_u64() as usize == window_index)
                    .delete();

                if let Some(transformer) = self.transformer_functions.get(&transformer) {
                    if let Some(validation) = &transformer.input_validation {
                        let validated = validation(ui);
                        if validated == TransformerState::Valid {
                            (transformer.transform_function)(
                                &window.editor_data.selected,
                                &window.window_tile,
                            );

                            if let Some(w) = self.window_list.get_by_id_mut(window_index) {
                                w.trigger(EditorStateTrigger::TransformerDone)
                            }
                        } else if validated == TransformerState::Cancelled {
                            if let Some(w) = self.window_list.get_by_id_mut(window_index) {
                                w.trigger(EditorStateTrigger::TransformerCancelled)
                            }
                        }
                    } else {
                        (transformer.transform_function)(
                            &window.editor_data.selected,
                            &window.window_tile,
                        );

                        if let Some(w) = self.window_list.get_by_id_mut(window_index) {
                            w.trigger(EditorStateTrigger::TransformerDone)
                        }
                    }
                }
            }
        }
    }
}
