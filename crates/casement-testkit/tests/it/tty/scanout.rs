//! The scheduler against `FakeScanout` and the virtual clock (P1-D3).

#[test]
#[ignore = "P1-D3"]
fn thirty_six_thousand_idle_frames_miss_nothing() {
    todo!("P1-D3: 36,000 virtual frames at 60 Hz, FrameOutcome::OnTime every time");
}

#[test]
#[ignore = "P1-D3"]
fn scripted_slow_frames_miss_exactly_as_scripted() {
    todo!("P1-D3: RenderCostModel::Script with known slow frames, exact Missed counts");
}

#[test]
#[ignore = "P1-D3"]
fn a_busy_surface_defers_the_frame_to_the_next_vblank() {
    todo!("P1-D3: QueueOutcome::Busy, then Queued at the next vblank");
}

#[test]
#[ignore = "P1-D3"]
fn the_cursor_plane_decision_follows_the_caps() {
    todo!("P1-D3: hardware cursor when it fits, composited otherwise, on the fake");
}
