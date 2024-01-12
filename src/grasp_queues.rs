use grasp_proc_macros::GraspQueue;
use itertools::Itertools;
use mosaic::{
    capabilities::{
        process::ProcessCapability, ArchetypeSubject, CollageImportCapability, StringCapability,
    },
    internals::{
        pars, void, ComponentValuesBuilderSetter, MosaicCRUD, MosaicIO, Tile, TileFieldEmptyQuery,
        Value,
    },
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
                        .set("message", err.as_str().as_bytes())
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
                            )
                            .as_str()
                            .as_bytes(),
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

    fn process_window_transformer_queue(&mut self) {
        while let Some(request) = dequeue(WindowTransformerQueue, &self.editor_mosaic) {
            let transformer_id = request.get("transform").as_u64() as usize;
            let window_index = request.get("window_index").as_u64() as usize;

            self.editor_mosaic
                .get_all()
                .include_component("Error")
                .filter(|t| t.get("window").as_u64() as usize == window_index)
                .delete();

            if let Some(window) = self
                .window_list
                .windows
                .iter()
                .find(|w| w.window_tile.id == window_index)
            {
                let instance = window.editor_data.selected.first().unwrap();

                if let Some(transformer_template) = window.transformer_mosaic.get(transformer_id) {
                    let fn_name = transformer_template
                        .get_component("Transformer")
                        .unwrap()
                        .get("self")
                        .as_s32()
                        .to_string();

                    if let Some(func) = self.transformer_functions.get(&fn_name) {
                        let error_count = self.validate_against_template(
                            &transformer_template,
                            instance,
                            &fn_name,
                            window_index as u64,
                            &mut vec![],
                        );

                        if error_count > 0 {
                            return;
                        }

                        let mut args = transformer_template
                            .iter()
                            .get_arrows_from()
                            .get_targets()
                            .map(|t| Label(&t).query())
                            .collect_vec();

                        args.extend(
                            transformer_template
                                .iter()
                                .get_arrows_into()
                                .get_sources()
                                .map(|t| Label(&t).query()),
                        );

                        let proc_tile = window
                            .document_mosaic
                            .create_process(
                                &fn_name,
                                args.iter().map(|s| s.as_str()).collect_vec().as_slice(),
                            )
                            .unwrap();

                        println!(
                            "Passing parameter {} = {}",
                            args.first().unwrap(),
                            instance.id
                        );
                        window
                            .document_mosaic
                            .pass_process_parameter(&proc_tile, args.first().unwrap(), instance)
                            .unwrap();

                        let result = (func)(&proc_tile);
                        fs::write(
                            "MyEnum.cs",
                            &window.document_mosaic.get_string_value(&result).unwrap(),
                        )
                        .unwrap();
                        /* let input_templates =
                            transformer_template.iter().get_arrows_into().get_sources();

                        let params = input_templates
                            .clone()
                            .map(|i| Label(&i).query())
                            .collect_vec();
                        let params2 = params.iter().map(|s| s.as_str()).collect_vec();
                        let str_slice: &[&str] = &params2;

                        let proc_tile = window
                            .document_mosaic
                            .create_process(&fn_name, str_slice)
                            .unwrap();

                        for input_template in input_templates.collect_vec() {
                            let input_name = Label(&input_template).query();

                            let input_instance = window.editor_data.selected.first().unwrap();

                            let mut error_count = 0;

                            for validation in input_template.iter().get_descriptors() {
                                let maybe_error = match validation.component.to_string().as_str() {
                                    "NoArrowsInto" => {
                                        if !input_instance
                                            .iter()
                                            .get_arrows_into()
                                            .collect_vec()
                                            .is_empty()
                                        {
                                            Some(format!("This tile requires no that no arrows go into it for transformer {} to work.", fn_name))
                                        } else {
                                            None
                                        }
                                    }
                                    "HasArrowsInto" => {
                                        if input_instance
                                            .iter()
                                            .get_arrows_into()
                                            .collect_vec()
                                            .is_empty()
                                        {
                                            Some(format!("This tile requires at least one arrow going into it for transformer {} to work.", fn_name))
                                        } else {
                                            None
                                        }
                                    }
                                    "HasComponent" => {
                                        let necessary_comp_name =
                                            validation.get("self").as_s32().to_string();

                                        if input_instance
                                            .get_component(&necessary_comp_name)
                                            .is_none()
                                        {
                                            Some(format!("Component '{}' required on this tile for transformer {} to work.", necessary_comp_name, fn_name))
                                        } else {
                                            None
                                        }
                                    }
                                    "HasArrowsFrom" => {
                                        if input_instance
                                            .iter()
                                            .get_arrows_from()
                                            .collect_vec()
                                            .is_empty()
                                        {
                                            Some(format!("This tile requires at least one arrow going from it for transformer {} to work.", fn_name))
                                        } else {
                                            None
                                        }
                                    }
                                    "NoArrowsFrom" => {
                                        if !input_instance
                                            .iter()
                                            .get_arrows_from()
                                            .collect_vec()
                                            .is_empty()
                                        {
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
                                            .set("message", err.as_str().as_bytes())
                                            .set("target", input_instance.id as u64)
                                            .set("window", window_index as u64)
                                            .ok(),
                                    )
                                });
                            }

                            if error_count > 0 {
                                return;
                            }

                            window
                                .document_mosaic
                                .pass_process_parameter(&proc_tile, &input_name, input_instance)
                                .unwrap();

                            let result = (func)(&proc_tile);
                            fs::write(
                                "MyEnum.cs",
                                &window.document_mosaic.get_string_value(&result).unwrap(),
                            )
                            .unwrap();
                        } */
                    }
                }
            }

            request.iter().delete();
        }
    }
}
