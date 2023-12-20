use std::{path::Path, time::Instant};

use imgui::{ConfigFlags, ImString, Ui, WindowFlags};
use log::info;
use sdl2::video::GLProfile;

use crate::{grasp_common::read_window_size, seq::SeqWriter};

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
    let window = if let Err((w, h)) = read_window_size() {
        info!("Loading with width {} and height {}", w, h);
        video
            .window(app_name, w as u32, h as u32)
            .position_centered()
            .fullscreen_desktop()
            .resizable()
            .opengl()
            .allow_highdpi()
            .build()
            .unwrap()
    } else {
        let rect = video.display_bounds(0).unwrap();

        video
            .window(app_name, rect.width() as u32, rect.height() as u32)
            .fullscreen_desktop()
            .set_window_flags(WindowFlags::empty().bits())
            .resizable()
            .opengl()
            .allow_highdpi()
            .build()
            .unwrap()
    };

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

    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut last_frame = Instant::now();
    let mut should_quit = false;

    'running: loop {
        use sdl2::event::Event;
        use sdl2::keyboard::Keycode;

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,

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

        imgui_sdl2.prepare_frame(imgui.io_mut(), &window, &event_pump.mouse_state());

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

        imgui_sdl2.prepare_render(ui, &window);

        renderer.render(&mut imgui);
        window.gl_swap_window();

        ::std::thread::sleep(::std::time::Duration::new(0, 1_000_000_000u32 / 60));

        if should_quit {
            break 'running;
        }
    }
}

pub fn set_window_focus(name: &str) {
    unsafe {
        imgui::sys::igSetWindowFocus_Str(ImString::new(name).as_ptr());
    }
}
