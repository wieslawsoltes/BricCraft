//! CPU atlas-packing benchmark; this does not measure GPU frame rate.
//! cargo run --release -p briccraft-render-data --example benchmark
use briccraft_parts::PALETTE;
use briccraft_render_data::SceneAtlas;
use briccraft_world::{DEFAULT_SEED, Instance, World};
use std::hint::black_box;
use std::time::Instant;

// Linux process CPU time separates packing work from descheduling on a shared
// runner. Other platforms still report the portable Instant wall-clock times.
#[cfg(target_os = "linux")]
fn cpu_nanoseconds() -> Option<u64> {
    #[repr(C)]
    struct Timespec {
        seconds: std::ffi::c_long,
        nanoseconds: std::ffi::c_long,
    }
    unsafe extern "C" {
        fn clock_gettime(clock: std::ffi::c_int, time: *mut Timespec) -> std::ffi::c_int;
    }
    let mut value = Timespec {
        seconds: 0,
        nanoseconds: 0,
    };
    // CLOCK_PROCESS_CPUTIME_ID is 2 in Linux's <bits/time.h>. The pointer is
    // valid and points to the C timespec layout for the target's long width.
    let result = unsafe { clock_gettime(2, &mut value) };
    (result == 0).then(|| value.seconds as u64 * 1_000_000_000 + value.nanoseconds as u64)
}

#[cfg(not(target_os = "linux"))]
fn cpu_nanoseconds() -> Option<u64> {
    None
}

fn measure(name: &str, world: &World) {
    let mut times = Vec::with_capacity(20);
    let mut cpu_times = Vec::with_capacity(20);
    let mut bytes = 0;
    let mut occupied = 0;
    for index in 0..22 {
        let started = Instant::now();
        let cpu_started = cpu_nanoseconds();
        let atlas = SceneAtlas::build(world, &[], None).expect("valid benchmark scene");
        let cpu_elapsed = cpu_started
            .zip(cpu_nanoseconds())
            .map(|(a, b)| (b - a) as f64 / 1_000_000.0);
        let milliseconds = started.elapsed().as_secs_f64() * 1000.0;
        bytes = atlas.stats.texture_bytes;
        occupied = atlas.stats.occupied_cells;
        black_box(&atlas);
        if index >= 2 {
            times.push(milliseconds);
            if let Some(cpu_elapsed) = cpu_elapsed {
                cpu_times.push(cpu_elapsed);
            }
        }
    }
    times.sort_by(f64::total_cmp);
    let median = (times[9] + times[10]) * 0.5;
    cpu_times.sort_by(f64::total_cmp);
    let median_cpu = if cpu_times.len() == 20 {
        format!("{:.6}", (cpu_times[9] + cpu_times[10]) * 0.5)
    } else {
        "null".to_owned()
    };
    println!(
        "{{\"benchmark\":\"{name}\",\"backend\":\"CPU atlas packing\",\"samples\":20,\"median_ms\":{median:.6},\"median_cpu_ms\":{median_cpu},\"minimum_ms\":{:.6},\"p95_ms\":{:.6},\"parts\":{},\"occupied_cells\":{occupied},\"active_cells\":{},\"atlas_bytes\":{bytes}}}",
        times[0],
        times[18],
        world.len(),
        world.occupancy().len()
    );
}

fn main() {
    let world = World::generate(DEFAULT_SEED);
    measure("generated_island", &world);
    let mut dense = World::empty(1);
    let parts: Vec<_> = (0..100_000_u32)
        .map(|id| Instance {
            id,
            part_id: 20,
            color: (id as usize % PALETTE.len()) as u8,
            position: [
                (id % 96) as i32,
                (id / (96 * 96)) as i32,
                (id / 96 % 96) as i32,
            ],
            rotation: 0,
        })
        .collect();
    dense
        .restore(&parts)
        .expect("dense fixture fits the active volume");
    measure("dense_100k", &dense);
}
