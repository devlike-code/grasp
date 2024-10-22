use std::{env, fs};

use mosaic::internals::{pars, ComponentValuesBuilderSetter, MosaicIO};

use crate::{core::structures::grasp_queues, grasp_queues::WindowRenameRequestQueue};

use super::{foundation::GraspEditorState, windows::GraspEditorWindow};

pub trait SaveFileCapability {
    fn save_file(&mut self);
    fn save_file_as(&mut self);
}

impl SaveFileCapability for GraspEditorState {
    fn save_file(&mut self) {
        if let Some(focused_window) = self.window_list.get_focused_mut() {
            assert!(focused_window.document_mosaic.id != self.editor_mosaic.id);

            focused_window.save_file();
        }
    }

    fn save_file_as(&mut self) {
        if let Some(focused_window) = self.window_list.get_focused_mut() {
            assert!(focused_window.document_mosaic.id != self.editor_mosaic.id);

            focused_window.save_file_as();
        }
    }
}

impl SaveFileCapability for GraspEditorWindow {
    fn save_file(&mut self) {
        if self.path.is_none() {
            self.save_file_as();
            if let Some(path) = self.path.clone() {
                GraspEditorState::prepend_recent(path);
            }
        } else {
            let document = self.document_mosaic.save();
            fs::write(self.path.clone().unwrap(), document).unwrap();

            self.changed = false;
        }
    }

    fn save_file_as(&mut self) {
        let document = self.document_mosaic.save();
        if let Some(file) = rfd::FileDialog::new()
            .add_filter("Mosaic", &["mos"])
            .set_directory(env::current_dir().unwrap())
            .save_file()
        {
            fs::write(file.clone(), document).unwrap();
            grasp_queues::enqueue(
                WindowRenameRequestQueue,
                self.editor_mosaic.new_object(
                    "WindowRenameRequest",
                    pars()
                        .set("id", self.window_tile.id as u64)
                        .set("index", self.window_list_index as u64)
                        .set("name", file.file_name().unwrap().to_str().unwrap())
                        .ok(),
                ),
            );
            self.path = Some(file.clone());
            self.changed = false;
        }
    }
}
