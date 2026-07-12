#![windows_subsystem = "windows"]

use std::env;
use std::error::Error;

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
use winit::event_loop::ActiveEventLoop;

#[derive(Deserialize)]
struct Config {
    debug_console: bool,
    sensitivity: f32,
}

#[cfg(target_os = "windows")]
fn init_console() {
    let config_content = fs::read_to_string("config.toml").unwrap_or_default();
    let mut config: Config = toml::from_str(&config_content).unwrap_or(Config { debug_console: true, sensitivity: 1.0 });
    let is_special = is_special();
    if is_special {config.debug_console = false};

    if config.debug_console {
        unsafe {
            if AllocConsole() != 0 {
                let _ = std::process::Command::new("cmd").arg("/c").status();
            }
        }

        colored::control::set_override(true);

        println!("{}", "Hello, audiophile!".bright_cyan().bold());
        println!("Audio sensitivity: {}", config.sensitivity);
    }
}

fn main() {
    init_console();
    handle_config_creation();

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
                        state.window.set_cursor_visible(true)
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

fn is_special() -> bool {
    let exe_path = env::current_exe().expect(&format!("{}", "Failed to get current exe path".red().bold()));
    let current_dir = exe_path.parent().expect(&format!("{}", "Failed to get exe directory".red().bold()));

    let dir_str = current_dir.to_string_lossy().to_lowercase();

    let special_folders = [
        "\\desktop",
        "\\downloads",
        "\\documents",
        "\\pictures",
        "\\videos",
        "\\music",
        "\\загрузки",
        "\\рабочий стол",
        "\\документы",
    ];

    let is_special = special_folders.iter().any(|folder| dir_str.contains(folder));
    is_special
}

fn handle_config_creation() {
    let exe_path = env::current_exe().expect(&format!("{}", "Failed to get current exe path".red().bold()));
    let current_dir = exe_path.parent().expect(&format!("{}", "Failed to get exe directory".red().bold()));
    let is_special = is_special();

    if !is_special {
        let config_path = current_dir.join("config.toml");

        if !config_path.exists() {
            let default_toml = r#"debug_console = false
sensitivity = 1.0
"#;
            if let Err(e) = fs::write(config_path, default_toml) {
                let str = format!("Unable to create config.toml: {}", e.to_string().italic());
                println!("{}", str.red().bold());
            } else {
                println!("{}", "Created default config.toml in program folder.".green().bold());
            }
        }
    }
}