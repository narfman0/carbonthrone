use carbonthrone::temporal_flux::{HIGH_FLUX_THRESHOLD, MAX_FLUX, TemporalFlux};

#[test]
fn flux_starts_at_zero() {
    let flux = TemporalFlux::default();
    assert_eq!(flux.flux, 0);
}

#[test]
fn flux_add_increases_value() {
    let mut flux = TemporalFlux::default();
    flux.add(30);
    assert_eq!(flux.flux, 30);
    flux.add(20);
    assert_eq!(flux.flux, 50);
}

#[test]
fn flux_add_clamps_to_max() {
    let mut flux = TemporalFlux::default();
    flux.add(200);
    assert_eq!(flux.flux, MAX_FLUX);
}

#[test]
fn flux_reset_zeroes_value() {
    let mut flux = TemporalFlux::default();
    flux.add(75);
    flux.reset();
    assert_eq!(flux.flux, 0);
}

#[test]
fn is_overloaded_false_below_max() {
    let mut flux = TemporalFlux::default();
    flux.add(99);
    assert!(!flux.is_overloaded());
}

#[test]
fn is_overloaded_true_at_max() {
    let mut flux = TemporalFlux::default();
    flux.add(MAX_FLUX);
    assert!(flux.is_overloaded());
}

#[test]
fn hit_penalty_zero_at_or_below_threshold() {
    let mut flux = TemporalFlux::default();
    flux.add(HIGH_FLUX_THRESHOLD);
    assert_eq!(flux.hit_penalty(), 0.0);
}

#[test]
fn hit_penalty_zero_when_flux_is_zero() {
    let flux = TemporalFlux::default();
    assert_eq!(flux.hit_penalty(), 0.0);
}

#[test]
fn hit_penalty_max_at_flux_100() {
    let mut flux = TemporalFlux::default();
    flux.add(MAX_FLUX);
    let penalty = flux.hit_penalty();
    // Should ramp to 0.20 at max flux.
    assert!(
        (penalty - 0.20).abs() < 1e-5,
        "expected 0.20 penalty at max flux, got {penalty}"
    );
}

#[test]
fn hit_penalty_half_at_midpoint() {
    let mut flux = TemporalFlux::default();
    // Midpoint between HIGH_FLUX_THRESHOLD (75) and MAX_FLUX (100) is 87 or 88.
    flux.flux = 87; // (87-75)/25 * 0.20 = 12/25 * 0.20 ≈ 0.096
    let penalty = flux.hit_penalty();
    assert!(
        penalty > 0.0 && penalty < 0.20,
        "expected partial penalty, got {penalty}"
    );
}

#[test]
fn hit_penalty_increases_with_flux() {
    let mut flux_low = TemporalFlux::default();
    flux_low.flux = 80;

    let mut flux_high = TemporalFlux::default();
    flux_high.flux = 95;

    assert!(
        flux_high.hit_penalty() > flux_low.hit_penalty(),
        "higher flux should have higher penalty"
    );
}
