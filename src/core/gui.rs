#![allow(dead_code)]

use imgui::sys::cty::c_char;
pub mod docking;
pub mod imgui_keys;
pub mod windowing;
pub mod components;

pub fn calc_text_size<T: AsRef<str>>(text: T) -> [f32; 2] {
    calc_text_size_with_opts(text, false, -1.0)
}


pub fn calc_text_size_with_opts<T: AsRef<str>>(    
    text: T,
    hide_text_after_double_hash: bool,
    wrap_width: f32,
) -> [f32; 2] {
    let mut out = imgui::sys::ImVec2::zero();
    let text = text.as_ref();

    unsafe {
        let start = text.as_ptr();
        let end = start.add(text.len());

        imgui::sys::igCalcTextSize(
            &mut out,
            start as *const c_char,
            end as *const c_char,
            hide_text_after_double_hash,
            wrap_width,
        )
    };
    out.into()
}
