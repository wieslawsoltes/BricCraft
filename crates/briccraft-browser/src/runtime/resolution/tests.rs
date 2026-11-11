use super::*;

fn frames(
    controller: &mut ResolutionController,
    counter: &mut u64,
    count: usize,
    dt: f64,
    quality: u8,
) -> ResolutionStats {
    let mut result = controller.stats();
    for _ in 0..count {
        *counter += 1;
        result = controller.update(dt, *counter, [1280.0, 800.0], 1.0, quality, true);
    }
    result
}

#[test]
fn auto_starts_at_native_desktop_density_with_requested_antialiasing() {
    let mut controller = ResolutionController::new();
    let stats = controller.update(0.0, 0, [1280.0, 800.0], 1.0, 1, true);
    assert_eq!([stats.pixel_width, stats.pixel_height], [1280, 800]);
    assert_eq!(stats.samples, 2);
    assert_eq!(stats.effective_quality, 1);
    assert_eq!(stats.mode_name(), "auto");
}

#[test]
fn compilation_stalls_do_not_destroy_initial_quality() {
    let mut controller = ResolutionController::new();
    let mut counter = 0;
    controller.update(0.0, counter, [1280.0, 800.0], 1.0, 2, true);
    let compiled = frames(&mut controller, &mut counter, 4, 4.0, 2);
    assert_eq!(compiled.pixel_budget, 1_024_000);
    assert_eq!(compiled.samples, 2);
    assert_eq!(compiled.effective_quality, 2);
}

#[test]
fn sustained_pressure_reduces_samples_then_effects_before_density() {
    let mut controller = ResolutionController::new();
    let mut counter = 0;
    controller.update(0.0, counter, [1280.0, 800.0], 1.0, 2, true);
    frames(&mut controller, &mut counter, 4, 0.1, 2);
    let aa = frames(&mut controller, &mut counter, 1, 1.0, 2);
    assert_eq!(aa.samples, 1);
    assert_eq!(aa.effective_quality, 2);
    assert_eq!(aa.pixel_budget, 1_024_000);
    let effects = frames(&mut controller, &mut counter, 1, 1.0, 2);
    assert_eq!(effects.effective_quality, 1);
    assert_eq!(effects.pixel_budget, 1_024_000);
    let floor = frames(&mut controller, &mut counter, 100, 1.0, 2);
    assert!(floor.pixel_width >= 958 && floor.pixel_height >= 598);
    assert_eq!(floor.pixel_budget, 576_000);
    assert_eq!(floor.minimum_pixels, 576_000);
}

#[test]
fn quality_locks_native_density_detail_and_samples_even_when_slow() {
    let mut controller = ResolutionController::new();
    controller.set_mode(1);
    controller.set_samples(4);
    let mut counter = 0;
    controller.update(0.0, counter, [1280.0, 800.0], 1.0, 2, true);
    let stats = frames(&mut controller, &mut counter, 40, 2.0, 2);
    assert_eq!([stats.pixel_width, stats.pixel_height], [1280, 800]);
    assert_eq!(stats.effective_quality, 2);
    assert_eq!(stats.samples, 4);
    assert_eq!(stats.mode_name(), "quality");
}

#[test]
fn performance_is_explicit_and_switching_back_restores_density() {
    let mut controller = ResolutionController::new();
    controller.set_mode(2);
    let fast = controller.update(0.0, 0, [1280.0, 800.0], 1.0, 2, true);
    assert!(fast.pixel_width >= 510 && fast.pixel_height >= 318);
    assert_eq!(fast.samples, 1);
    assert_eq!(fast.effective_quality, 0);
    controller.set_mode(0);
    let auto = controller.update(0.0, 0, [1280.0, 800.0], 1.0, 2, true);
    assert_eq!(auto.pixel_width, 1280);
    assert_eq!(auto.samples, 2);
    assert_eq!(auto.effective_quality, 2);
}

#[test]
fn recovery_requires_sustained_headroom_and_restores_density_first() {
    let mut controller = ResolutionController::new();
    controller.set_mode(2);
    let mut counter = 0;
    let initial = controller.update(0.0, counter, [1280.0, 800.0], 1.0, 1, true);
    let brief = frames(&mut controller, &mut counter, 90, 1.0 / 60.0, 1);
    assert_eq!(brief.pixel_budget, initial.pixel_budget);
    let recovery = frames(&mut controller, &mut counter, 180, 1.0 / 60.0, 1);
    assert!(recovery.pixel_budget > initial.pixel_budget);
    assert_eq!(recovery.effective_quality, 0);
    assert_eq!(recovery.samples, 1);
}

#[test]
fn quality_and_samples_never_exceed_the_players_request() {
    let mut controller = ResolutionController::new();
    controller.set_samples(4);
    let mut counter = 0;
    controller.update(0.0, counter, [1280.0, 800.0], 1.0, 2, true);
    frames(&mut controller, &mut counter, 300, 1.0 / 60.0, 2);
    controller.set_samples(1);
    let low = controller.update(0.0, counter, [1280.0, 800.0], 1.0, 0, true);
    assert_eq!(low.samples, 1);
    assert_eq!(low.effective_quality, 0);
    let stable = frames(&mut controller, &mut counter, 600, 1.0 / 60.0, 0);
    assert_eq!(stable.samples, 1);
    assert_eq!(stable.effective_quality, 0);
}

#[test]
fn idle_menu_time_does_not_reduce_scene_quality() {
    let mut controller = ResolutionController::new();
    controller.update(0.0, 0, [1280.0, 800.0], 1.0, 1, true);
    let mut counter = 0;
    frames(&mut controller, &mut counter, 4, 0.02, 1);
    for _ in 0..20 {
        controller.update(3.0, counter, [1280.0, 800.0], 1.0, 1, false);
    }
    let stats = controller.stats();
    assert_eq!(stats.pixel_budget, 1_024_000);
    assert_eq!(stats.samples, 2);
    assert_eq!(stats.effective_quality, 1);
}

#[test]
fn mobile_retina_and_large_desktops_stay_bounded() {
    for mode in [0, 1, 2] {
        for (size, dpi) in [
            ([390.0, 844.0], 3.0),
            ([844.0, 390.0], 3.0),
            ([3840.0, 2160.0], 2.0),
            ([7680.0, 4320.0], 2.0),
            ([1.0, 1.0], 1.0),
            ([f64::NAN, f64::INFINITY], f64::NAN),
        ] {
            let mut controller = ResolutionController::new();
            controller.set_mode(mode);
            let stats = controller.update(0.0, 0, size, dpi, 2, true);
            assert!(stats.render_scale > 0.0 && stats.render_scale <= 1.0);
            assert!(stats.effect_width <= 4096 && stats.effect_height <= 4096);
            assert!(
                stats.effect_width as u64 * stats.effect_height as u64 <= stats.pixel_budget as u64
            );
            assert!(stats.pixel_budget <= 8_294_400);
            assert!(stats.pixel_width > 0 && stats.pixel_height > 0);
        }
    }
}

#[test]
fn preferences_and_orientation_changes_apply_immediately() {
    let mut controller = ResolutionController::new();
    controller.set_mode(1);
    controller.set_samples(4);
    let portrait = controller.update(0.0, 0, [390.0, 844.0], 2.0, 2, true);
    let landscape = controller.update(0.0, 0, [844.0, 390.0], 2.0, 2, true);
    assert_eq!([portrait.pixel_width, portrait.pixel_height], [780, 1688]);
    assert_eq!([landscape.pixel_width, landscape.pixel_height], [1688, 780]);
    assert_eq!(landscape.samples, 4);
    controller.set_mode(255);
    controller.set_samples(255);
    let sane = controller.update(0.0, 0, [844.0, 390.0], 2.0, 9, true);
    assert_eq!(sane.mode_name(), "auto");
    assert_eq!(sane.requested_samples, 1);
    assert_eq!(sane.requested_quality, 2);
}

#[test]
fn resizing_preserves_the_observed_cost_tradeoff() {
    let mut controller = ResolutionController::new();
    let mut counter = 0;
    controller.update(0.0, counter, [1280.0, 800.0], 1.0, 2, true);
    frames(&mut controller, &mut counter, 100, 1.0, 2);
    let resized = controller.update(0.02, counter, [1281.0, 800.0], 1.0, 2, true);
    assert_eq!(resized.effective_quality, 0);
    assert_eq!(resized.samples, 1);
    assert!(resized.pixel_budget < 580_000);
    let smaller = controller.update(0.02, counter, [640.5, 400.0], 1.0, 2, true);
    assert_eq!(smaller.effective_quality, 0);
    assert_eq!(smaller.samples, 1);
    assert!((smaller.pixel_budget as i64 * 4 - resized.pixel_budget as i64).abs() <= 3);
    controller.set_samples(4);
    let requested = controller.update(0.02, counter, [640.5, 400.0], 1.0, 2, true);
    assert_eq!(requested.samples, 4);
    assert_eq!(requested.effective_quality, 2);
}
