//! The browser host is an adapter: the world and simulation have no browser dependency.
#![cfg_attr(target_os = "emscripten", no_main)]
mod application;
mod bridge;
mod commands;
mod design_bridge;
mod design_host;
mod diagnostics;
mod gestures;
mod map_bridge;
mod map_host;
mod map_image;
mod runtime;
mod viewport;
mod world_bridge;
mod world_host;

#[cfg(not(target_os = "emscripten"))]
fn main() {
    println!("Build the browser application with scripts/build-browser.sh");
}
