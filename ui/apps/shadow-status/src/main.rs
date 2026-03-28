mod color;
mod layout;
mod model;
mod primitives;
mod scene;

#[cfg(not(target_os = "linux"))]
mod desktop;
#[cfg(not(target_os = "linux"))]
mod renderer;
#[cfg(not(target_os = "linux"))]
mod text;
#[cfg(target_os = "linux")]
mod wayland;

#[cfg(not(target_os = "linux"))]
fn main() {
    desktop::run();
}

#[cfg(target_os = "linux")]
fn main() {
    wayland::run();
}
