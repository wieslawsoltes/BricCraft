# BricCraft

A brick-built sandbox for the browser, written in Rust with FerroUI's declarative UI and Skia renderer.

## Development

FerroUI is pinned as a git submodule at `FerroUI/`. Clone with `git clone --recurse-submodules https://github.com/wieslawsoltes/BricCraft.git`.

The application is being implemented as reusable crates for the part catalogue, world construction, mechanical simulation, GPU rendering, game state, declarative UI, and browser hosting. GitHub Pages is the browser distribution target.
