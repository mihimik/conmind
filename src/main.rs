#![windows_subsystem = "windows"]

pub mod render_context;
mod render;
mod audio;

use render::State;

use winit::{
    event::*,
    event_loop::EventLoop,
    window::Window,
};

use serde::Deserialize;
use std::fs;
use colored::Colorize;
use windows_sys::Win32::System::Console::AllocConsole;

#[derive(Deserialize)]
struct Config {
    debug_console: bool,
}

#[cfg(target_os = "windows")]
fn init_console() {
    let config_content = fs::read_to_string("config.toml").unwrap_or_default();
    let config: Config = toml::from_str(&config_content).unwrap_or(Config { debug_console: false });

    if config.debug_console {
        unsafe {
            if AllocConsole() != 0 {
                let _ = std::process::Command::new("cmd").arg("/c").status();
            }
        }
        println!("{}", "Hello, audiophile!".bright_cyan().bold());
    }
}

fn main() {
    init_console();

    let event_loop = EventLoop::new().unwrap();
    let window = event_loop.create_window(Window::default_attributes()
        .with_title("ConMind Visualizer")
        .with_inner_size(winit::dpi::PhysicalSize::new(800, 600)))
        .unwrap();

    let mut state = pollster::block_on(State::new(window));
    let mut last_frame_inst = std::time::Instant::now();

    event_loop.run(move |event, elwt| {
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => elwt.exit(),

                WindowEvent::KeyboardInput {
                    event: KeyEvent {
                        logical_key: winit::keyboard::Key::Named(winit::keyboard::NamedKey::F11),
                        state: ElementState::Pressed,
                        ..
                    },
                    ..
                } => {
                    let is_fullscreen = state.window.fullscreen().is_some();
                    if is_fullscreen {
                        state.window.set_fullscreen(None);
                    } else {
                        state.window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
                        state.window.set_cursor_visible(false)
                    }
                }

                WindowEvent::Resized(physical_size) => {
                    state.resize(physical_size);
                }

                WindowEvent::RedrawRequested => {
                    let _ = state.render();
                }
                _ => {}
            },

            Event::AboutToWait => {
                let now = std::time::Instant::now();
                let dt = now.duration_since(last_frame_inst);
                last_frame_inst = now;

                state.update(dt);
                state.window.request_redraw();
            }
            _ => {}
        }

    }).unwrap();

    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--was-restarted") {
        println!("\n[Program Finished] Press Enter to close console...");
        let mut s = String::new();
        let _ = std::io::stdin().read_line(&mut s);
    }
}