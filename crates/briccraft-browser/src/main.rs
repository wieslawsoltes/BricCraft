//! The browser host is an adapter: the world and simulation have no browser dependency.
#![cfg_attr(target_os = "emscripten", no_main)]
mod application;
mod bridge;
mod runtime;
mod viewport;

#[cfg(not(target_os = "emscripten"))]
fn main() {
    println!("Build the browser application with scripts/build-browser.sh");
}
