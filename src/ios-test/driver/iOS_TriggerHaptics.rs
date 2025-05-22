#[cfg(any(target_os = "ios", target_os = "macos"))]
use objc2::MainThreadMarker;
#[cfg(any(target_os = "ios"))]
use objc2_ui_kit::UIImpactFeedbackGenerator;

#[cfg(any(target_os = "ios"))]
pub unsafe fn trigger_haptic() {
    let mtm = MainThreadMarker::new().expect("must be on the main thread");
    let generator = UIImpactFeedbackGenerator::new(mtm);
    let intensity = 0.75;
    generator.prepare();
    generator.impactOccurredWithIntensity(intensity);
}