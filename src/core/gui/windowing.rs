use std::{
    collections::HashMap,
    fs::File,
    io::Read,
    path::Path,
    sync::{Arc, Mutex},
    time::Instant,
};

use imgui::{
    sys::{igGetWindowDrawList, igImMax, igImMin, igImRotate, ImVec2, ImVec4},
    ConfigFlags, ImString, Ui, WindowFlags,
};
use lazy_static::lazy_static;
use log::info;

use gl::types::GLvoid;
use sdl2::video::GLProfile;

use crate::{core::math::Vec2, seq::SeqWriter};

pub const LOAD_TEXTURE_EVENT: u32 = 10101;

lazy_static! {
    static ref TEXTURES: Arc<Mutex<HashMap<String, u32>>> =
        Arc::new(Mutex::new(HashMap::default()));
}

pub fn get_texture(name: &str) -> u32 {
    *TEXTURES.lock().unwrap().get(name).unwrap()
}

fn load_image(data: &Vec<u8>, width: i32, height: i32, depth: i32) -> u32 {
    let mut texture_id: gl::types::GLuint = 0;
    unsafe {
        gl::GenTextures(1, &mut texture_id);
        gl::BindTexture(gl::TEXTURE_2D, texture_id);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
        gl::GenerateMipmap(gl::TEXTURE_2D);
        info!("Loaded texture with size {}x{}x{}", width, height, depth);
        let depth_channels = if depth == 3 {
            gl::RGB as i32
        } else {
            gl::RGBA as i32
        };
        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            depth_channels,
            width,
            height,
            0,
            depth_channels as u32,
            gl::UNSIGNED_BYTE,
            data.as_ptr() as *const GLvoid,
        );
        gl::BindTexture(gl::TEXTURE_2D, 0);
    }

    texture_id
}

pub fn gl_smooth() {
    unsafe {
        gl::Enable(gl::POLYGON_SMOOTH);
        gl::Hint(gl::POLYGON_SMOOTH_HINT, gl::NICEST);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::ONE, gl::SRC_ALPHA_SATURATE);
    }
}

pub fn gl_not_smooth() {
    unsafe {
        gl::Disable(gl::POLYGON_SMOOTH);
    }
}

pub fn load_image_asset(name: &str, asset: &str) {
    let mut asset = File::open(asset).unwrap();
    let mut buffer: Vec<u8> = vec![];
    asset.read_to_end(&mut buffer).unwrap();

    match stb_image::image::load_from_memory(buffer.as_slice()) {
        stb_image::image::LoadResult::ImageU8(i) => {
            let texture = load_image(&i.data, i.width as i32, i.height as i32, i.depth as i32);
            assert!(texture != 0);
            TEXTURES.lock().unwrap().insert(name.to_string(), texture);
        }
        stb_image::image::LoadResult::Error(err) => panic!("{:?}", err),
        stb_image::image::LoadResult::ImageF32(_) => todo!(),
    }
}

fn gui_rotate_start() -> usize {
    unsafe { igGetWindowDrawList().as_mut().unwrap().VtxBuffer.Size as usize }
}

fn gui_rotate(center: ImVec2, c: f32, s: f32) -> ImVec2 {
    unsafe {
        let mut p_out: ImVec2 = ImVec2::zero();
        igImRotate(&mut p_out as *mut _, center, c, s);
        p_out
    }
}

fn gui_rotate_center(rot_id: usize) -> ImVec2 {
    let mut l = ImVec2::new(f32::MAX, f32::MAX);
    let mut u = ImVec2::new(-f32::MAX, -f32::MAX);

    unsafe {
        let buf = igGetWindowDrawList().as_ref().unwrap().VtxBuffer;
        for i in rot_id..(buf.Size as usize) {
            let p = buf.Data.add(i);
            let pi = p.as_mut().unwrap().pos;
            igImMin(&mut l as *mut _, l, pi);
            igImMax(&mut u as *mut _, u, pi);
        }

        ImVec2::new((l.x + u.x) / 2.0, (l.y + u.y) / 2.0)
    }
}

fn gui_rotate_end(rot_id: usize, rad: f32) {
    fn get_center(rot: ImVec2, center: ImVec2) -> ImVec2 {
        let rv2: Vec2 = rot.into();
        let cv2: Vec2 = center.into();
        let iv2: ImVec2 = (rv2 - cv2).into();

        iv2
    }

    let center = gui_rotate_center(rot_id);
    let s = rad.sin();
    let c = rad.cos();

    let center = get_center(gui_rotate(center, c, s), center);

    unsafe {
        let buf = igGetWindowDrawList().as_ref().unwrap().VtxBuffer;
        for i in rot_id..(buf.Size as usize) {
            let p = buf.Data.add(i);
            let pi = p.as_mut().unwrap().pos;
            p.as_mut().unwrap().pos = get_center(gui_rotate(pi, c, s), center);
        }
    }
}

pub fn gui_draw_image(name: &str, size: [f32; 2], pos: [f32; 2], rot: f32, opacity: f32) {
    unsafe {
        let local_pos = ImVec2::new(pos[0] - size[0] / 2.0, pos[1] - size[1] / 2.0);
        imgui::sys::igSetCursorPos(local_pos);

        let rot_id = gui_rotate_start();
        imgui::sys::igImage(
            get_texture(name) as *mut _,
            ImVec2::new(size[0], size[1]),
            ImVec2::new(0.0, 0.0),
            ImVec2::new(1.0, 1.0),
            ImVec4::new(1.0, 1.0, 1.0, opacity),
            ImVec4::new(0.0, 0.0, 0.0, 0.0),
        );
        gui_rotate_end(rot_id, rot);
    }
}

pub fn run_main_forever<F: FnMut(&Ui, &mut bool)>(mut update: F) {
    let writer = SeqWriter::new();

    let sdl_context = sdl2::init().unwrap();
    let video = sdl_context.video().unwrap();

    env_logger::builder()
        .format(move |_buf, record| {
            writer.send(record);
            Ok(())
        })
        .filter_level(log::LevelFilter::Debug)
        .init();

    let app_name = "GRASP";
    let window = video
        .window(app_name, 100, 100)
        .set_window_flags(WindowFlags::empty().bits())
        .maximized()
        .resizable()
        .opengl()
        .allow_highdpi()
        .build()
        .unwrap();

    let _gl_context = window
        .gl_create_context()
        .expect("Couldn't create GL context");
    gl::load_with(|s| video.gl_get_proc_address(s) as _);
    let gl_attr = video.gl_attr();

    gl_attr.set_context_version(3, 3);
    gl_attr.set_context_profile(GLProfile::Core);

    let mut imgui = imgui::Context::create();
    imgui.set_ini_filename(Some(Path::new("grasp.ini").to_path_buf()));
    imgui.io_mut().config_flags = ConfigFlags::DOCKING_ENABLE | ConfigFlags::VIEWPORTS_ENABLE;
    imgui.io_mut().config_windows_move_from_title_bar_only = true;
    let mut imgui_sdl2 = imgui_sdl2::ImguiSdl2::new(&mut imgui, &window);

    let renderer =
        imgui_opengl_renderer::Renderer::new(&mut imgui, |s| video.gl_get_proc_address(s) as _);

    let canvas = window.into_canvas().build().unwrap();

    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut last_frame = Instant::now();
    let mut should_quit = false;

    load_image_asset("dot", "assets//dot.png");
    load_image_asset("[dot]", "assets//selected-dot.png");
    load_image_asset("arrow", "assets//arrow.png");
    load_image_asset("[arrow]", "assets//selected-arrow.png");
    load_image_asset("arrowhead", "assets//arrowhead.png");

    'running: loop {
        use sdl2::event::Event;

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'running,

                Event::KeyDown {
                    scancode: Some(fkey),
                    ..
                } if fkey as usize >= 58 && fkey as usize <= 69 => {
                    imgui.io_mut().keys_down[fkey as usize + 514] = true;
                }

                Event::KeyUp {
                    scancode: Some(fkey),
                    ..
                } if fkey as usize >= 58 && fkey as usize <= 69 => {
                    imgui.io_mut().keys_down[fkey as usize + 514] = false;
                }

                e => {
                    imgui_sdl2.handle_event(&mut imgui, &e);
                }
            }
        }

        imgui_sdl2.prepare_frame(imgui.io_mut(), canvas.window(), &event_pump.mouse_state());

        let now = Instant::now();
        let delta = now - last_frame;
        let delta_s = delta.as_secs() as f32 + delta.subsec_nanos() as f32 / 1_000_000_000.0;
        last_frame = now;
        imgui.io_mut().delta_time = delta_s;

        let ui = imgui.frame();
        ui.dockspace_over_main_viewport();

        update(ui, &mut should_quit);

        //ui.show_demo_window(&mut true);

        unsafe {
            gl::ClearColor(0.2, 0.2, 0.2, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }

        imgui_sdl2.prepare_render(ui, canvas.window());

        renderer.render(&mut imgui);

        canvas.window().gl_swap_window();

        ::std::thread::sleep(::std::time::Duration::new(0, 1_000_000_000u32 / 60));

        if should_quit {
            break 'running;
        }
    }
}

pub fn gui_set_window_focus(name: &str) {
    unsafe {
        imgui::sys::igSetWindowFocus_Str(ImString::new(name).as_ptr());
    }
}
