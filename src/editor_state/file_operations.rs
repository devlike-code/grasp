use std::{env, fs};

use mosaic::internals::MosaicIO;

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
            fs::write(file, document).unwrap();
            self.changed = false;
        }
    }
}
