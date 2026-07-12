# ConMind

*MilkDrop/Windows XP style sound visualizer.*

![output](https://github.com/user-attachments/assets/f774c7e1-4592-45ea-9038-1f529064e802)

**ConMind** is a program that visualizes sound using mathematical formulas. The project's goal is to create a visualizer with unusual effects, reminiscent of programs from the 2000s.

Program has the following advantages:

1. Lightweight
2. Quick run
3. Supports all audio output devices

# Restrictions

The current version does not support microphone input and Linux.

# Quick Start

Download the [latest release](https://github.com/mihimik/conmind/releases/tag/v0.2.0) and run the downloaded EXE file.

If you want to build the project yourself, make sure you have Rust installed.
Clone the repository:
```
git clone https://github.com/mihimik/conmind
cd conmind
```
Build and run:
```
cargo run --release
```
*Note: The --release flag is critical for visualizers, as without optimizations, audio processing may be worse.*

# Program Configs
Configs allow you to customize the program to your taste or device features.
At this moment you can customize the following elements:
1. Debug Console. You can see program logs and errors.
2. Sensitivity. Shows how strong the visual effects will be.

## Ways to unlock configs
### Manual

1. Add file "config.toml" in program folder
2. Put this text into:
```toml
debug_console = true
sensitivity = 1
```
Note: You can set any values for sensitivity, even negative ones. The creator doesn't clip your experimentation — discover the glitchy neon-green palette for yourself!

### Automatic

1. Put the program to any non-system folder
2. Launch it

*Note: The console will be opened only at first launch if you don't change it in the settings.*
