use std::ptr::{null, null_mut};

use imgui::{
    sys::{
        ImGuiDir_Down, ImGuiDir_Left, ImGuiDir_None, ImGuiDir_Right, ImGuiDir_Up,
        ImGuiDockNodeFlags_None, ImGuiDockNodeFlags_PassthruCentralNode, ImGuiID, ImVec2,
    },
    Id,
};

#[derive(Clone, Copy)]
pub struct GuiViewport {
    pub ptr: *mut imgui::sys::ImGuiViewport,
}

impl GuiViewport {
    pub fn get_main_viewport() -> Self {
        Self {
            ptr: unsafe { imgui::sys::igGetMainViewport() },
        }
    }

    pub fn size(&self) -> ImVec2 {
        unsafe { self.ptr.as_ref().unwrap().Size }
    }
}

#[non_exhaustive]
#[derive(Copy, Clone, Debug)]
pub enum GuiDockNodeFlags {
    None = ImGuiDockNodeFlags_None as isize,
    Passthru = ImGuiDockNodeFlags_PassthruCentralNode as isize,
}

#[non_exhaustive]
#[derive(Copy, Clone, Debug)]
pub enum GuiDir {
    None = ImGuiDir_None as isize,
    Up = ImGuiDir_Up as isize,
    Down = ImGuiDir_Down as isize,
    Left = ImGuiDir_Left as isize,
    Right = ImGuiDir_Right as isize,
}

impl From<&GuiViewport> for *mut imgui::sys::ImGuiViewport {
    fn from(value: &GuiViewport) -> Self {
        value.ptr
    }
}

impl From<&GuiViewport> for *const imgui::sys::ImGuiViewport {
    fn from(value: &GuiViewport) -> Self {
        value.ptr
    }
}

pub struct GuiDockspace {
    pub id: u32,
}

pub fn new_id<S: AsRef<str>>(s: S) -> u32 {
    unsafe {
        let mut buffer = vec![];

        buffer.extend(s.as_ref().as_bytes());
        buffer.push(b'\0');

        let name = buffer.as_ptr() as *const _;

        imgui::sys::igGetID_Str(name)
    }
}

impl GuiDockspace {
    pub fn over_viewport(viewport: &GuiViewport, flags: GuiDockNodeFlags) -> GuiDockspace {
        let dockspace_id =
            unsafe { imgui::sys::igDockSpaceOverViewport(viewport.ptr, flags as i32, null()) };

        GuiDockspace { id: dockspace_id }
    }

    pub fn new(id: u32, position: ImVec2, flags: GuiDockNodeFlags) -> GuiDockspace {
        unsafe {
            let id = imgui::sys::igDockSpace(id, position, flags as i32, null());
            GuiDockspace { id }
        }
    }

    pub fn remove_node(&self) {
        unsafe {
            imgui::sys::igDockBuilderRemoveNode(self.id);
        }
    }

    pub fn add_node(&self, flags: GuiDockNodeFlags) {
        unsafe {
            imgui::sys::igDockBuilderAddNode(self.id, flags as i32);
        }
    }

    pub fn set_node_size(&self, size: ImVec2) {
        unsafe {
            imgui::sys::igDockBuilderSetNodeSize(self.id, size);
        }
    }

    pub fn set_node_position(&self, pos: ImVec2) {
        unsafe {
            imgui::sys::igDockBuilderSetNodePos(self.id, pos);
        }
    }

    pub fn split(&mut self, dir: GuiDir, ratio: f32) -> ImGuiID {
        unsafe {
            imgui::sys::igDockBuilderSplitNode(
                self.id,
                dir as i32,
                ratio,
                null_mut(),
                &mut self.id as *mut u32,
            )
        }
    }

    pub fn dock_window<S: AsRef<str>>(&mut self, s: S, panel: ImGuiID) {
        let mut buffer = vec![];

        buffer.extend(s.as_ref().as_bytes());
        buffer.push(b'\0');

        let name = buffer.as_ptr() as *const _;

        unsafe {
            imgui::sys::igDockBuilderDockWindow(name, panel);
        }
    }

    pub fn finish(mut self) {
        unsafe {
            imgui::sys::igDockBuilderFinish(self.id);
        }
    }
}
