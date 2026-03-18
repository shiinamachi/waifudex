use std::collections::BTreeMap;

use waifudex_mascot::MascotParamValue;

use crate::contracts::runtime::RuntimeStatus;

const LOOP_DURATION_SECONDS: f32 = 4.2;
const IDLE_LOOP_SECONDS: f32 = 14.0;
const THINKING_LOOP_SECONDS: f32 = 12.0;
const QUESTION_LOOP_SECONDS: f32 = 10.0;
const COMPLETE_LOOP_SECONDS: f32 = 8.0;
const TAU: f32 = std::f32::consts::PI * 2.0;

#[derive(Clone, Copy)]
struct CanonicalParam {
    name: &'static str,
    x: f32,
    y: f32,
}

#[derive(Clone, Copy)]
struct MotionLayer {
    name: &'static str,
    axis: Axis,
    amplitude: f32,
    period: Option<f32>,
    phase: f32,
}

#[derive(Clone, Copy)]
enum Axis {
    X,
    Y,
}

fn param(name: &'static str, x: f32, y: f32) -> CanonicalParam {
    CanonicalParam { name, x, y }
}

fn breathing_layers() -> [MotionLayer; 1] {
    [MotionLayer {
        name: "ParamBreath",
        axis: Axis::Y,
        amplitude: 0.6,
        period: None,
        phase: 0.0,
    }]
}

fn clamp01(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

fn clamp_signed(value: f32) -> f32 {
    value.clamp(-1.0, 1.0)
}

fn loop_time(elapsed_seconds: f32, duration: f32) -> f32 {
    let looped = elapsed_seconds % duration;
    if looped < 0.0 {
        looped + duration
    } else {
        looped
    }
}

fn evaluate_layer(layer: MotionLayer, elapsed_seconds: f32) -> f32 {
    let period = layer.period.unwrap_or(LOOP_DURATION_SECONDS);
    let angle = ((elapsed_seconds / period) + layer.phase) * TAU;
    angle.sin() * layer.amplitude
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn base_pose(status: RuntimeStatus) -> Vec<CanonicalParam> {
    match status {
        RuntimeStatus::Idle => {
            // Handled by create_idle_targets; kept for exhaustiveness
            vec![]
        }
        RuntimeStatus::CodexNotInstalled => vec![
            param("ParamBodyAngleX", -0.6, 0.0),
            param("ParamEyeOpen", 0.0, 0.75),
            param("ParamMouthOpenY", 0.0, 0.15),
            param("ParamBreath", 0.0, 0.5),
        ],
        RuntimeStatus::Thinking => {
            // Handled by create_thinking_targets; kept for exhaustiveness
            vec![]
        }
        RuntimeStatus::Coding => {
            // Handled by create_thinking_targets; kept for exhaustiveness
            vec![]
        }
        RuntimeStatus::Question => {
            // Handled by create_question_targets; kept for exhaustiveness
            vec![]
        }
        RuntimeStatus::Complete => {
            // Handled by create_complete_targets; kept for exhaustiveness
            vec![]
        }
    }
}

fn layers(status: RuntimeStatus) -> Vec<MotionLayer> {
    match status {
        RuntimeStatus::Thinking => {
            // Handled by create_thinking_targets; kept for exhaustiveness
            vec![]
        }
        RuntimeStatus::Idle => {
            // Handled by create_idle_targets; kept for exhaustiveness
            vec![]
        }
        RuntimeStatus::Coding => {
            // Handled by create_thinking_targets; kept for exhaustiveness
            vec![]
        }
        RuntimeStatus::Question => {
            // Handled by create_question_targets; kept for exhaustiveness
            vec![]
        }
        RuntimeStatus::Complete => {
            // Handled by create_complete_targets; kept for exhaustiveness
            vec![]
        }
        RuntimeStatus::CodexNotInstalled => breathing_layers().to_vec(),
    }
}

// ---------------------------------------------------------------------------
// Thinking: keyframe-interpolated phase animation
// ---------------------------------------------------------------------------

const PHASE_PARAM_COUNT: usize = 11;

type PhaseParams = [CanonicalParam; PHASE_PARAM_COUNT];

// ---------------------------------------------------------------------------
// Idle: bored, listless keyframe animation
// ---------------------------------------------------------------------------

const IDLE_PHASE_TIMES: [f32; 4] = [0.0, 4.0 / 14.0, 7.0 / 14.0, 11.0 / 14.0];

fn idle_keyframes() -> [PhaseParams; 4] {
    [
        // Phase 1 (0–4s): Zoning out, absent-minded
        [
            param("ParamAngleX", 0.6, 0.0),
            param("ParamBodyAngleX", 0.35, 0.0),
            param("ParamEyeOpen", 0.0, 0.55),
            param("ParamEyeMove", 0.2, 0.0),
            param("ParamMouthOpenY", 0.0, 0.06),
            param("ParamMouthSmile", 0.0, 0.0),
            param("ParamArmLeft", 0.5, 0.0),
            param("ParamArmRight", 0.5, 0.0),
            param("ParamBreath", 0.0, 0.5),
            param("ParamBodyXMove", 0.5, 0.0),
            param("ParamTailMove", 0.5, 0.0),
        ],
        // Phase 2 (4–7s): Deep sigh, "haah..."
        [
            param("ParamAngleX", -0.4, 0.0),
            param("ParamBodyAngleX", -0.5, 0.0),
            param("ParamEyeOpen", 0.0, 0.3),
            param("ParamEyeMove", 0.35, 0.0),
            param("ParamMouthOpenY", 0.0, 0.5),
            param("ParamMouthSmile", 0.0, 0.0),
            param("ParamArmLeft", 0.5, 0.0),
            param("ParamArmRight", 0.5, 0.0),
            param("ParamBreath", 0.0, 0.9),
            param("ParamBodyXMove", 0.3, 0.0),
            param("ParamTailMove", 0.1, 0.0),
        ],
        // Phase 3 (7–11s): Looking around lazily, searching for something
        [
            param("ParamAngleX", -0.8, 0.0),
            param("ParamBodyAngleX", -0.3, 0.0),
            param("ParamEyeOpen", 0.0, 0.85),
            param("ParamEyeMove", 0.85, 0.0),
            param("ParamMouthOpenY", 0.0, 0.12),
            param("ParamMouthSmile", 0.0, 0.25),
            param("ParamArmLeft", 0.5, 0.0),
            param("ParamArmRight", 0.25, 0.0),
            param("ParamBreath", 0.0, 0.5),
            param("ParamBodyXMove", 0.72, 0.0),
            param("ParamTailMove", 0.9, 0.0),
        ],
        // Phase 4 (11–14s): Settling back to boredom
        [
            param("ParamAngleX", 0.25, 0.0),
            param("ParamBodyAngleX", 0.15, 0.0),
            param("ParamEyeOpen", 0.0, 0.65),
            param("ParamEyeMove", 0.42, 0.0),
            param("ParamMouthOpenY", 0.0, 0.1),
            param("ParamMouthSmile", 0.0, 0.0),
            param("ParamArmLeft", 0.5, 0.0),
            param("ParamArmRight", 0.5, 0.0),
            param("ParamBreath", 0.0, 0.5),
            param("ParamBodyXMove", 0.5, 0.0),
            param("ParamTailMove", 0.4, 0.0),
        ],
    ]
}

/// All periods must divide evenly into IDLE_LOOP_SECONDS (14.0) for seamless looping.
fn idle_overlay_layers() -> [MotionLayer; 7] {
    [
        // Slow lazy head drift
        MotionLayer {
            name: "ParamAngleX",
            axis: Axis::X,
            amplitude: 0.18,
            period: Some(7.0),
            phase: 0.15,
        },
        // Body sway
        MotionLayer {
            name: "ParamBodyAngleX",
            axis: Axis::X,
            amplitude: 0.1,
            period: Some(7.0),
            phase: 0.4,
        },
        // Drowsy eye flutter
        MotionLayer {
            name: "ParamEyeOpen",
            axis: Axis::Y,
            amplitude: 0.2,
            period: Some(3.5),
            phase: 0.2,
        },
        // Slow deep breathing
        MotionLayer {
            name: "ParamBreath",
            axis: Axis::Y,
            amplitude: 0.4,
            period: Some(3.5),
            phase: 0.0,
        },
        // Secondary breathing
        MotionLayer {
            name: "ParamBreath",
            axis: Axis::Y,
            amplitude: 0.15,
            period: Some(2.0),
            phase: 0.35,
        },
        // Lazy tail sway
        MotionLayer {
            name: "ParamTailMove",
            axis: Axis::X,
            amplitude: 0.35,
            period: Some(3.5),
            phase: 0.5,
        },
        // Wandering gaze
        MotionLayer {
            name: "ParamEyeMove",
            axis: Axis::X,
            amplitude: 0.2,
            period: Some(7.0),
            phase: 0.6,
        },
    ]
}

fn create_idle_targets(elapsed_seconds: f32) -> Vec<MascotParamValue> {
    let elapsed = loop_time(elapsed_seconds, IDLE_LOOP_SECONDS);
    let normalized = elapsed / IDLE_LOOP_SECONDS;

    let keyframes = idle_keyframes();
    let (from_idx, to_idx, progress) =
        find_keyframe_segment(normalized, &IDLE_PHASE_TIMES);
    let base = interpolate_phase_params(&keyframes[from_idx], &keyframes[to_idx], progress);

    let mut params = BTreeMap::<&'static str, CanonicalParam>::new();
    for p in base {
        params.insert(p.name, p);
    }

    for layer in idle_overlay_layers() {
        let current = params
            .get(layer.name)
            .copied()
            .unwrap_or_else(|| param(layer.name, 0.0, 0.0));
        let delta = evaluate_layer(layer, elapsed);
        let updated = match layer.axis {
            Axis::X => param(current.name, current.x + delta, current.y),
            Axis::Y => param(current.name, current.x, current.y + delta),
        };
        params.insert(layer.name, updated);
    }

    params
        .into_values()
        .map(|p| clamp_canonical(p.name, p.x, p.y))
        .flat_map(resolve_actual_params)
        .collect()
}

// ---------------------------------------------------------------------------
// Thinking: keyframe-interpolated phase animation
// ---------------------------------------------------------------------------

const THINKING_PHASE_TIMES: [f32; 4] = [0.0, 4.0 / 12.0, 6.5 / 12.0, 9.5 / 12.0];

fn thinking_keyframes() -> [PhaseParams; 4] {
    [
        // Phase 1 (0–4s): Arms crossed, deep pondering
        [
            param("ParamAngleX", -0.85, 0.0),
            param("ParamBodyAngleX", -0.5, 0.0),
            param("ParamEyeOpen", 0.0, 0.65),
            param("ParamEyeMove", 0.15, 0.0),
            param("ParamMouthOpenY", 0.0, 0.1),
            param("ParamMouthSmile", 0.0, 0.0),
            param("ParamArmLeft", 0.92, 0.0),
            param("ParamArmRight", 0.08, 0.0),
            param("ParamBreath", 0.0, 0.55),
            param("ParamBodyXMove", 0.25, 0.0),
            param("ParamTailMove", 0.5, 0.0),
        ],
        // Phase 2 (4–6.5s): "Hmm?" moment of insight
        [
            param("ParamAngleX", 0.7, 0.0),
            param("ParamBodyAngleX", 0.3, 0.0),
            param("ParamEyeOpen", 0.0, 0.98),
            param("ParamEyeMove", 0.85, 0.0),
            param("ParamMouthOpenY", 0.0, 0.45),
            param("ParamMouthSmile", 0.0, 0.5),
            param("ParamArmLeft", 0.65, 0.0),
            param("ParamArmRight", 0.35, 0.0),
            param("ParamBreath", 0.0, 0.7),
            param("ParamBodyXMove", 0.75, 0.0),
            param("ParamTailMove", 0.9, 0.0),
        ],
        // Phase 3 (6.5–9.5s): Head scratch, "nah…"
        [
            param("ParamAngleX", 0.8, 0.0),
            param("ParamBodyAngleX", 0.55, 0.0),
            param("ParamEyeOpen", 0.0, 0.72),
            param("ParamEyeMove", 0.2, 0.0),
            param("ParamMouthOpenY", 0.0, 0.6),
            param("ParamMouthSmile", 0.0, 0.0),
            param("ParamArmLeft", 0.5, 0.0),
            param("ParamArmRight", 0.95, 0.0),
            param("ParamBreath", 0.0, 0.3),
            param("ParamBodyXMove", 0.8, 0.0),
            param("ParamTailMove", 0.05, 0.0),
        ],
        // Phase 4 (9.5–12s): Transition back to crossed-arms pose
        [
            param("ParamAngleX", -0.4, 0.0),
            param("ParamBodyAngleX", -0.15, 0.0),
            param("ParamEyeOpen", 0.0, 0.8),
            param("ParamEyeMove", 0.4, 0.0),
            param("ParamMouthOpenY", 0.0, 0.2),
            param("ParamMouthSmile", 0.0, 0.0),
            param("ParamArmLeft", 0.82, 0.0),
            param("ParamArmRight", 0.15, 0.0),
            param("ParamBreath", 0.0, 0.5),
            param("ParamBodyXMove", 0.35, 0.0),
            param("ParamTailMove", 0.35, 0.0),
        ],
    ]
}

/// Sinusoidal overlays added on top of keyframe interpolation for liveness.
/// All periods must divide evenly into THINKING_LOOP_SECONDS for seamless looping.
fn thinking_overlay_layers() -> [MotionLayer; 8] {
    [
        MotionLayer {
            name: "ParamAngleX",
            axis: Axis::X,
            amplitude: 0.2,
            period: Some(3.0),
            phase: 0.1,
        },
        MotionLayer {
            name: "ParamBodyAngleX",
            axis: Axis::X,
            amplitude: 0.12,
            period: Some(4.0),
            phase: 0.25,
        },
        MotionLayer {
            name: "ParamEyeOpen",
            axis: Axis::Y,
            amplitude: 0.15,
            period: Some(2.0),
            phase: 0.15,
        },
        MotionLayer {
            name: "ParamBreath",
            axis: Axis::Y,
            amplitude: 0.35,
            period: Some(3.0),
            phase: 0.0,
        },
        MotionLayer {
            name: "ParamBreath",
            axis: Axis::Y,
            amplitude: 0.12,
            period: Some(1.5),
            phase: 0.4,
        },
        MotionLayer {
            name: "ParamTailMove",
            axis: Axis::X,
            amplitude: 0.3,
            period: Some(2.4),
            phase: 0.3,
        },
        MotionLayer {
            name: "ParamEyeMove",
            axis: Axis::X,
            amplitude: 0.18,
            period: Some(6.0),
            phase: 0.5,
        },
        MotionLayer {
            name: "ParamMouthOpenY",
            axis: Axis::Y,
            amplitude: 0.12,
            period: Some(2.0),
            phase: 0.6,
        },
    ]
}

fn find_keyframe_segment(t: f32, times: &[f32]) -> (usize, usize, f32) {
    let n = times.len();
    let mut from_idx = 0;
    for i in (0..n).rev() {
        if t >= times[i] {
            from_idx = i;
            break;
        }
    }
    let to_idx = (from_idx + 1) % n;
    let t_start = times[from_idx];
    let t_end = if to_idx == 0 { 1.0 } else { times[to_idx] };
    let duration = t_end - t_start;
    let progress = if duration > 0.0 {
        (t - t_start) / duration
    } else {
        0.0
    };
    (from_idx, to_idx, progress)
}

fn interpolate_phase_params(from: &PhaseParams, to: &PhaseParams, t: f32) -> Vec<CanonicalParam> {
    let t = smoothstep(t);
    from.iter()
        .zip(to.iter())
        .map(|(a, b)| param(a.name, lerp(a.x, b.x, t), lerp(a.y, b.y, t)))
        .collect()
}

fn create_thinking_targets(elapsed_seconds: f32) -> Vec<MascotParamValue> {
    let elapsed = loop_time(elapsed_seconds, THINKING_LOOP_SECONDS);
    let normalized = elapsed / THINKING_LOOP_SECONDS;

    let keyframes = thinking_keyframes();
    let (from_idx, to_idx, progress) =
        find_keyframe_segment(normalized, &THINKING_PHASE_TIMES);
    let base = interpolate_phase_params(&keyframes[from_idx], &keyframes[to_idx], progress);

    let mut params = BTreeMap::<&'static str, CanonicalParam>::new();
    for p in base {
        params.insert(p.name, p);
    }

    for layer in thinking_overlay_layers() {
        let current = params
            .get(layer.name)
            .copied()
            .unwrap_or_else(|| param(layer.name, 0.0, 0.0));
        let delta = evaluate_layer(layer, elapsed);
        let updated = match layer.axis {
            Axis::X => param(current.name, current.x + delta, current.y),
            Axis::Y => param(current.name, current.x, current.y + delta),
        };
        params.insert(layer.name, updated);
    }

    params
        .into_values()
        .map(|p| clamp_canonical(p.name, p.x, p.y))
        .flat_map(resolve_actual_params)
        .collect()
}

// ---------------------------------------------------------------------------
// Question: knock-knock, curious question animation
// ---------------------------------------------------------------------------

const QUESTION_PHASE_TIMES: [f32; 4] = [0.0, 2.0 / 10.0, 4.0 / 10.0, 7.0 / 10.0];

fn question_keyframes() -> [PhaseParams; 4] {
    [
        // Phase 1 (0–2s): Leaning in, approaching the screen
        [
            param("ParamAngleX", 0.0, 0.0),
            param("ParamBodyAngleX", 0.65, 0.0),
            param("ParamEyeOpen", 0.0, 0.95),
            param("ParamEyeMove", 0.5, 0.0),
            param("ParamMouthOpenY", 0.0, 0.08),
            param("ParamMouthSmile", 0.0, 0.25),
            param("ParamArmLeft", 0.5, 0.0),
            param("ParamArmRight", 0.85, 0.0),
            param("ParamBreath", 0.0, 0.7),
            param("ParamBodyXMove", 0.75, 0.0),
            param("ParamTailMove", 0.8, 0.0),
        ],
        // Phase 2 (2–4s): Knocking gesture — arm extended, body bouncing forward
        [
            param("ParamAngleX", 0.3, 0.0),
            param("ParamBodyAngleX", 0.75, 0.0),
            param("ParamEyeOpen", 0.0, 0.98),
            param("ParamEyeMove", 0.6, 0.0),
            param("ParamMouthOpenY", 0.0, 0.2),
            param("ParamMouthSmile", 0.0, 0.3),
            param("ParamArmLeft", 0.5, 0.0),
            param("ParamArmRight", 0.98, 0.0),
            param("ParamBreath", 0.0, 0.55),
            param("ParamBodyXMove", 0.82, 0.0),
            param("ParamTailMove", 0.92, 0.0),
        ],
        // Phase 3 (4–7s): Asking the question — curious head tilt, mouth open
        [
            param("ParamAngleX", -0.8, 0.0),
            param("ParamBodyAngleX", 0.3, 0.0),
            param("ParamEyeOpen", 0.0, 0.96),
            param("ParamEyeMove", 0.25, 0.0),
            param("ParamMouthOpenY", 0.0, 0.65),
            param("ParamMouthSmile", 0.0, 0.12),
            param("ParamArmLeft", 0.5, 0.0),
            param("ParamArmRight", 0.5, 0.0),
            param("ParamBreath", 0.0, 0.5),
            param("ParamBodyXMove", 0.6, 0.0),
            param("ParamTailMove", 0.2, 0.0),
        ],
        // Phase 4 (7–10s): Waiting for answer, settling back
        [
            param("ParamAngleX", -0.4, 0.0),
            param("ParamBodyAngleX", 0.12, 0.0),
            param("ParamEyeOpen", 0.0, 0.88),
            param("ParamEyeMove", 0.4, 0.0),
            param("ParamMouthOpenY", 0.0, 0.3),
            param("ParamMouthSmile", 0.0, 0.25),
            param("ParamArmLeft", 0.5, 0.0),
            param("ParamArmRight", 0.5, 0.0),
            param("ParamBreath", 0.0, 0.52),
            param("ParamBodyXMove", 0.5, 0.0),
            param("ParamTailMove", 0.6, 0.0),
        ],
    ]
}

/// All periods must divide evenly into QUESTION_LOOP_SECONDS (10.0) for seamless looping.
fn question_overlay_layers() -> [MotionLayer; 7] {
    [
        // Head movement
        MotionLayer {
            name: "ParamAngleX",
            axis: Axis::X,
            amplitude: 0.18,
            period: Some(2.5),
            phase: 0.1,
        },
        // Body bounce (knocking energy)
        MotionLayer {
            name: "ParamBodyAngleX",
            axis: Axis::X,
            amplitude: 0.12,
            period: Some(2.0),
            phase: 0.2,
        },
        // Eager eye flutter
        MotionLayer {
            name: "ParamEyeOpen",
            axis: Axis::Y,
            amplitude: 0.15,
            period: Some(2.5),
            phase: 0.15,
        },
        // Breathing
        MotionLayer {
            name: "ParamBreath",
            axis: Axis::Y,
            amplitude: 0.35,
            period: Some(2.5),
            phase: 0.0,
        },
        // Secondary breathing
        MotionLayer {
            name: "ParamBreath",
            axis: Axis::Y,
            amplitude: 0.15,
            period: Some(2.0),
            phase: 0.45,
        },
        // Active tail wag
        MotionLayer {
            name: "ParamTailMove",
            axis: Axis::X,
            amplitude: 0.35,
            period: Some(2.0),
            phase: 0.3,
        },
        // Glancing eyes
        MotionLayer {
            name: "ParamEyeMove",
            axis: Axis::X,
            amplitude: 0.18,
            period: Some(5.0),
            phase: 0.5,
        },
    ]
}

fn create_question_targets(elapsed_seconds: f32) -> Vec<MascotParamValue> {
    let elapsed = loop_time(elapsed_seconds, QUESTION_LOOP_SECONDS);
    let normalized = elapsed / QUESTION_LOOP_SECONDS;

    let keyframes = question_keyframes();
    let (from_idx, to_idx, progress) =
        find_keyframe_segment(normalized, &QUESTION_PHASE_TIMES);
    let base = interpolate_phase_params(&keyframes[from_idx], &keyframes[to_idx], progress);

    let mut params = BTreeMap::<&'static str, CanonicalParam>::new();
    for p in base {
        params.insert(p.name, p);
    }

    for layer in question_overlay_layers() {
        let current = params
            .get(layer.name)
            .copied()
            .unwrap_or_else(|| param(layer.name, 0.0, 0.0));
        let delta = evaluate_layer(layer, elapsed);
        let updated = match layer.axis {
            Axis::X => param(current.name, current.x + delta, current.y),
            Axis::Y => param(current.name, current.x, current.y + delta),
        };
        params.insert(layer.name, updated);
    }

    params
        .into_values()
        .map(|p| clamp_canonical(p.name, p.x, p.y))
        .flat_map(resolve_actual_params)
        .collect()
}

// ---------------------------------------------------------------------------
// Complete: excited celebration animation
// ---------------------------------------------------------------------------

const COMPLETE_PHASE_TIMES: [f32; 4] = [0.0, 2.0 / 8.0, 4.5 / 8.0, 6.5 / 8.0];

fn complete_keyframes() -> [PhaseParams; 4] {
    [
        // Phase 1 (0–2s): "Yay!" — arms up, big smile, bouncing up
        [
            param("ParamAngleX", 0.5, 0.0),
            param("ParamBodyAngleX", 0.55, 0.0),
            param("ParamEyeOpen", 0.0, 0.98),
            param("ParamEyeMove", 0.75, 0.0),
            param("ParamMouthOpenY", 0.0, 0.65),
            param("ParamMouthSmile", 0.0, 0.95),
            param("ParamArmLeft", 0.98, 0.0),
            param("ParamArmRight", 0.98, 0.0),
            param("ParamBreath", 0.0, 0.8),
            param("ParamBodyXMove", 0.65, 0.0),
            param("ParamTailMove", 0.92, 0.0),
        ],
        // Phase 2 (2–4.5s): Happy sway — leaning to one side, still grinning
        [
            param("ParamAngleX", -0.7, 0.0),
            param("ParamBodyAngleX", -0.5, 0.0),
            param("ParamEyeOpen", 0.0, 0.8),
            param("ParamEyeMove", 0.2, 0.0),
            param("ParamMouthOpenY", 0.0, 0.3),
            param("ParamMouthSmile", 0.0, 0.9),
            param("ParamArmLeft", 0.82, 0.0),
            param("ParamArmRight", 0.78, 0.0),
            param("ParamBreath", 0.0, 0.55),
            param("ParamBodyXMove", 0.2, 0.0),
            param("ParamTailMove", 0.05, 0.0),
        ],
        // Phase 3 (4.5–6.5s): Proud pose — hands on hips, satisfied look
        [
            param("ParamAngleX", 0.6, 0.0),
            param("ParamBodyAngleX", 0.4, 0.0),
            param("ParamEyeOpen", 0.0, 0.88),
            param("ParamEyeMove", 0.8, 0.0),
            param("ParamMouthOpenY", 0.0, 0.15),
            param("ParamMouthSmile", 0.0, 0.85),
            param("ParamArmLeft", 0.05, 0.0),
            param("ParamArmRight", 0.05, 0.0),
            param("ParamBreath", 0.0, 0.5),
            param("ParamBodyXMove", 0.75, 0.0),
            param("ParamTailMove", 0.85, 0.0),
        ],
        // Phase 4 (6.5–8s): Bouncing back to celebration, winding up for next loop
        [
            param("ParamAngleX", 0.0, 0.0),
            param("ParamBodyAngleX", 0.25, 0.0),
            param("ParamEyeOpen", 0.0, 0.95),
            param("ParamEyeMove", 0.5, 0.0),
            param("ParamMouthOpenY", 0.0, 0.5),
            param("ParamMouthSmile", 0.0, 0.92),
            param("ParamArmLeft", 0.92, 0.0),
            param("ParamArmRight", 0.94, 0.0),
            param("ParamBreath", 0.0, 0.7),
            param("ParamBodyXMove", 0.5, 0.0),
            param("ParamTailMove", 0.88, 0.0),
        ],
    ]
}

/// All periods must divide evenly into COMPLETE_LOOP_SECONDS (8.0) for seamless looping.
fn complete_overlay_layers() -> [MotionLayer; 8] {
    [
        // Excited head bobbing
        MotionLayer {
            name: "ParamAngleX",
            axis: Axis::X,
            amplitude: 0.22,
            period: Some(2.0),
            phase: 0.0,
        },
        // Happy body bounce
        MotionLayer {
            name: "ParamBodyAngleX",
            axis: Axis::X,
            amplitude: 0.15,
            period: Some(1.6),
            phase: 0.15,
        },
        // Sparkling eye blinks
        MotionLayer {
            name: "ParamEyeOpen",
            axis: Axis::Y,
            amplitude: 0.15,
            period: Some(2.0),
            phase: 0.3,
        },
        // Excited breathing
        MotionLayer {
            name: "ParamBreath",
            axis: Axis::Y,
            amplitude: 0.4,
            period: Some(2.0),
            phase: 0.0,
        },
        // Secondary breath flutter
        MotionLayer {
            name: "ParamBreath",
            axis: Axis::Y,
            amplitude: 0.15,
            period: Some(1.0),
            phase: 0.5,
        },
        // Energetic tail wagging
        MotionLayer {
            name: "ParamTailMove",
            axis: Axis::X,
            amplitude: 0.4,
            period: Some(1.6),
            phase: 0.2,
        },
        // Wandering happy gaze
        MotionLayer {
            name: "ParamEyeMove",
            axis: Axis::X,
            amplitude: 0.2,
            period: Some(4.0),
            phase: 0.4,
        },
        // Mouth jitter from excitement
        MotionLayer {
            name: "ParamMouthOpenY",
            axis: Axis::Y,
            amplitude: 0.15,
            period: Some(1.6),
            phase: 0.6,
        },
    ]
}

fn create_complete_targets(elapsed_seconds: f32) -> Vec<MascotParamValue> {
    let elapsed = loop_time(elapsed_seconds, COMPLETE_LOOP_SECONDS);
    let normalized = elapsed / COMPLETE_LOOP_SECONDS;

    let keyframes = complete_keyframes();
    let (from_idx, to_idx, progress) =
        find_keyframe_segment(normalized, &COMPLETE_PHASE_TIMES);
    let base = interpolate_phase_params(&keyframes[from_idx], &keyframes[to_idx], progress);

    let mut params = BTreeMap::<&'static str, CanonicalParam>::new();
    for p in base {
        params.insert(p.name, p);
    }

    for layer in complete_overlay_layers() {
        let current = params
            .get(layer.name)
            .copied()
            .unwrap_or_else(|| param(layer.name, 0.0, 0.0));
        let delta = evaluate_layer(layer, elapsed);
        let updated = match layer.axis {
            Axis::X => param(current.name, current.x + delta, current.y),
            Axis::Y => param(current.name, current.x, current.y + delta),
        };
        params.insert(layer.name, updated);
    }

    params
        .into_values()
        .map(|p| clamp_canonical(p.name, p.x, p.y))
        .flat_map(resolve_actual_params)
        .collect()
}

// ---------------------------------------------------------------------------
// Parameter resolution
// ---------------------------------------------------------------------------

fn clamp_canonical(name: &'static str, x: f32, y: f32) -> CanonicalParam {
    match name {
        "ParamAngleX" | "ParamBodyAngleX" => param(name, clamp_signed(x), 0.0),
        "ParamEyeOpen" | "ParamBreath" | "ParamMouthOpenY" | "ParamMouthSmile" => {
            param(name, 0.0, clamp01(y))
        }
        "ParamArmLeft" | "ParamArmRight" | "ParamEyeMove" | "ParamBodyXMove"
        | "ParamTailMove" => param(name, clamp01(x), 0.0),
        _ => param(name, x, y),
    }
}

fn resolve_actual_params(param: CanonicalParam) -> Vec<MascotParamValue> {
    match param.name {
        "ParamAngleX" => vec![
            MascotParamValue {
                name: "Head:: Yaw-Pitch".to_string(),
                x: param.x,
                y: 0.0,
            },
            MascotParamValue {
                name: "Head:: Roll".to_string(),
                x: param.x * 0.35,
                y: param.x * 0.35,
            },
        ],
        "ParamBodyAngleX" => vec![
            MascotParamValue {
                name: "Body:: Yaw-Pitch".to_string(),
                x: param.x,
                y: 0.0,
            },
            MascotParamValue {
                name: "Body:: Roll".to_string(),
                x: param.x * 0.2,
                y: param.x * 0.2,
            },
        ],
        "ParamEyeOpen" => {
            let blink = clamp01(1.0 - param.y);
            vec![
                MascotParamValue {
                    name: "Eye:: Left:: Blink".to_string(),
                    x: blink,
                    y: blink,
                },
                MascotParamValue {
                    name: "Eye:: Right:: Blink".to_string(),
                    x: blink,
                    y: blink,
                },
            ]
        }
        "ParamBreath" => vec![MascotParamValue {
            name: "Breath".to_string(),
            x: param.y,
            y: param.y,
        }],
        "ParamMouthOpenY" => vec![MascotParamValue {
            name: "Mouth:: Shape".to_string(),
            x: param.y,
            y: param.y,
        }],
        "ParamMouthSmile" => vec![MascotParamValue {
            name: "Mouth:: Width".to_string(),
            x: param.y,
            y: param.y,
        }],
        "ParamArmLeft" => vec![MascotParamValue {
            name: "Arm:: Left:: Move".to_string(),
            x: param.x,
            y: 0.0,
        }],
        "ParamArmRight" => vec![MascotParamValue {
            name: "Arm:: Right:: Move".to_string(),
            x: param.x,
            y: 0.0,
        }],
        "ParamEyeMove" => vec![
            MascotParamValue {
                name: "Eye:: Left:: Move".to_string(),
                x: param.x,
                y: 0.0,
            },
            MascotParamValue {
                name: "Eye:: Right:: Move".to_string(),
                x: param.x,
                y: 0.0,
            },
        ],
        "ParamBodyXMove" => vec![MascotParamValue {
            name: "Body:: X:: Move".to_string(),
            x: param.x,
            y: 0.0,
        }],
        "ParamTailMove" => vec![MascotParamValue {
            name: "Tail:: Move".to_string(),
            x: param.x,
            y: 0.0,
        }],
        _ => Vec::new(),
    }
}

pub fn create_motion_targets(status: RuntimeStatus, elapsed_seconds: f32) -> Vec<MascotParamValue> {
    match status {
        RuntimeStatus::Idle => return create_idle_targets(elapsed_seconds),
        RuntimeStatus::Thinking | RuntimeStatus::Coding => {
            return create_thinking_targets(elapsed_seconds)
        }
        RuntimeStatus::Question => return create_question_targets(elapsed_seconds),
        RuntimeStatus::Complete => return create_complete_targets(elapsed_seconds),
        _ => {}
    }

    let elapsed = loop_time(elapsed_seconds, LOOP_DURATION_SECONDS);
    let mut params = BTreeMap::<&'static str, CanonicalParam>::new();

    for pose in base_pose(status) {
        params.insert(pose.name, pose);
    }

    for layer in layers(status) {
        let current = params
            .get(layer.name)
            .copied()
            .unwrap_or_else(|| param(layer.name, 0.0, 0.0));
        let delta = evaluate_layer(layer, elapsed);
        let updated = match layer.axis {
            Axis::X => param(current.name, current.x + delta, current.y),
            Axis::Y => param(current.name, current.x, current.y + delta),
        };
        params.insert(layer.name, updated);
    }

    params
        .into_values()
        .map(|param| clamp_canonical(param.name, param.x, param.y))
        .flat_map(resolve_actual_params)
        .collect()
}

#[cfg(test)]
mod mascot_motion_tests {
    use crate::contracts::runtime::RuntimeStatus;

    use super::{
        create_motion_targets, COMPLETE_LOOP_SECONDS, IDLE_LOOP_SECONDS, QUESTION_LOOP_SECONDS,
        THINKING_LOOP_SECONDS,
    };

    fn get_param<'a>(
        params: &'a [waifudex_mascot::MascotParamValue],
        name: &str,
    ) -> Option<&'a waifudex_mascot::MascotParamValue> {
        params.iter().find(|param| param.name == name)
    }

    #[test]
    fn thinking_loop_is_seamless_at_start_and_end() {
        let first = create_motion_targets(RuntimeStatus::Thinking, 0.0);
        let second = create_motion_targets(RuntimeStatus::Thinking, THINKING_LOOP_SECONDS);

        assert_eq!(first, second);
    }

    #[test]
    fn idle_and_thinking_have_distinct_target_sets() {
        let idle = create_motion_targets(RuntimeStatus::Idle, 1.05);
        let thinking = create_motion_targets(RuntimeStatus::Thinking, 1.05);

        assert_ne!(idle, thinking);
    }

    #[test]
    fn codex_not_installed_keeps_breathing_motion() {
        let first = create_motion_targets(RuntimeStatus::CodexNotInstalled, 0.0);
        let second =
            create_motion_targets(RuntimeStatus::CodexNotInstalled, std::f32::consts::FRAC_PI_2);

        assert_ne!(
            get_param(&first, "Breath").map(|param| param.x),
            get_param(&second, "Breath").map(|param| param.x)
        );
    }

    #[test]
    fn coding_uses_thinking_motion() {
        let coding = create_motion_targets(RuntimeStatus::Coding, 3.0);
        let thinking = create_motion_targets(RuntimeStatus::Thinking, 3.0);
        assert_eq!(coding, thinking);
    }

    #[test]
    fn question_loop_is_seamless_at_start_and_end() {
        let first = create_motion_targets(RuntimeStatus::Question, 0.0);
        let second = create_motion_targets(RuntimeStatus::Question, QUESTION_LOOP_SECONDS);
        assert_eq!(first, second);
    }

    #[test]
    fn question_has_knocking_phase_with_arm_extended() {
        // Phase 1 start (t=0): arm at 0.7; Phase 2 start (t=2): arm at 0.85
        let approach = create_motion_targets(RuntimeStatus::Question, 0.0);
        let knock = create_motion_targets(RuntimeStatus::Question, 2.0);

        let arm_approach = get_param(&approach, "Arm:: Right:: Move").unwrap().x;
        let arm_knock = get_param(&knock, "Arm:: Right:: Move").unwrap().x;
        assert!(
            arm_knock > arm_approach,
            "expected arm more extended during knock: knock={arm_knock}, approach={arm_approach}"
        );
    }

    #[test]
    fn question_has_curious_head_tilt_during_asking() {
        // Phase 2 start (t=2): head slightly positive; Phase 3 middle (t=5.5): head negative
        let knock = create_motion_targets(RuntimeStatus::Question, 2.0);
        let asking = create_motion_targets(RuntimeStatus::Question, 5.5);

        let head_knock = get_param(&knock, "Head:: Yaw-Pitch").unwrap().x;
        let head_asking = get_param(&asking, "Head:: Yaw-Pitch").unwrap().x;
        assert!(
            head_knock * head_asking < 0.0,
            "expected head direction to flip: knock={head_knock}, asking={head_asking}"
        );
    }

    #[test]
    fn question_motion_has_visible_variation() {
        let samples = [0.0_f32, 2.0, 4.0, 6.0, 8.0]
            .into_iter()
            .map(|time| create_motion_targets(RuntimeStatus::Question, time))
            .collect::<Vec<_>>();

        let range = |name: &str, field: fn(&waifudex_mascot::MascotParamValue) -> f32| {
            let vals: Vec<f32> = samples
                .iter()
                .filter_map(|params| get_param(params, name).map(&field))
                .collect();
            vals.iter().copied().fold(f32::NEG_INFINITY, f32::max)
                - vals.iter().copied().fold(f32::INFINITY, f32::min)
        };

        let head_range = range("Head:: Yaw-Pitch", |p| p.x);
        let body_range = range("Body:: Yaw-Pitch", |p| p.x);
        let arm_range = range("Arm:: Right:: Move", |p| p.x);

        assert!(
            head_range > 0.10,
            "expected question head range > 0.10, got {head_range}"
        );
        assert!(
            body_range > 0.06,
            "expected question body range > 0.06, got {body_range}"
        );
        assert!(
            arm_range > 0.15,
            "expected question arm range > 0.15, got {arm_range}"
        );
    }

    #[test]
    fn complete_loop_is_seamless_at_start_and_end() {
        let first = create_motion_targets(RuntimeStatus::Complete, 0.0);
        let second = create_motion_targets(RuntimeStatus::Complete, COMPLETE_LOOP_SECONDS);
        assert_eq!(first, second);
    }

    #[test]
    fn complete_has_big_smile_throughout() {
        // Smile should stay high across all phases
        let samples = [0.0_f32, 2.0, 4.0, 6.0]
            .into_iter()
            .map(|time| create_motion_targets(RuntimeStatus::Complete, time))
            .collect::<Vec<_>>();

        for (i, params) in samples.iter().enumerate() {
            let smile = get_param(params, "Mouth:: Width").unwrap().x;
            assert!(
                smile > 0.7,
                "expected smile > 0.7 at sample {i}, got {smile}"
            );
        }
    }

    #[test]
    fn complete_has_arms_up_celebration() {
        // Phase 1 (t=0): both arms should be raised high
        let celebration = create_motion_targets(RuntimeStatus::Complete, 0.0);
        let arm_l = get_param(&celebration, "Arm:: Left:: Move").unwrap().x;
        let arm_r = get_param(&celebration, "Arm:: Right:: Move").unwrap().x;
        assert!(
            arm_l > 0.7 && arm_r > 0.7,
            "expected both arms raised: left={arm_l}, right={arm_r}"
        );
    }

    #[test]
    fn complete_motion_has_energetic_variation() {
        let samples = [0.0_f32, 1.5, 3.0, 5.0, 7.0]
            .into_iter()
            .map(|time| create_motion_targets(RuntimeStatus::Complete, time))
            .collect::<Vec<_>>();

        let range = |name: &str, field: fn(&waifudex_mascot::MascotParamValue) -> f32| {
            let vals: Vec<f32> = samples
                .iter()
                .filter_map(|params| get_param(params, name).map(&field))
                .collect();
            vals.iter().copied().fold(f32::NEG_INFINITY, f32::max)
                - vals.iter().copied().fold(f32::INFINITY, f32::min)
        };

        let head_range = range("Head:: Yaw-Pitch", |p| p.x);
        let tail_range = range("Tail:: Move", |p| p.x);
        let body_range = range("Body:: Yaw-Pitch", |p| p.x);

        assert!(
            head_range > 0.10,
            "expected complete head range > 0.10, got {head_range}"
        );
        assert!(
            tail_range > 0.15,
            "expected complete tail range > 0.15, got {tail_range}"
        );
        assert!(
            body_range > 0.06,
            "expected complete body range > 0.06, got {body_range}"
        );
    }

    #[test]
    fn thinking_targets_actual_aka_parameter_names() {
        let params = create_motion_targets(RuntimeStatus::Thinking, 0.0);

        assert!(get_param(&params, "Head:: Yaw-Pitch").is_some());
        assert!(get_param(&params, "Head:: Roll").is_some());
        assert!(get_param(&params, "Body:: Yaw-Pitch").is_some());
        assert!(get_param(&params, "Body:: Roll").is_some());
        assert!(get_param(&params, "Eye:: Left:: Blink").is_some());
        assert!(get_param(&params, "Eye:: Right:: Blink").is_some());
        assert!(get_param(&params, "Breath").is_some());
        assert!(get_param(&params, "Arm:: Left:: Move").is_some());
        assert!(get_param(&params, "Arm:: Right:: Move").is_some());
        assert!(get_param(&params, "Eye:: Left:: Move").is_some());
        assert!(get_param(&params, "Eye:: Right:: Move").is_some());
        assert!(get_param(&params, "Body:: X:: Move").is_some());
        assert!(get_param(&params, "Tail:: Move").is_some());
        assert!(get_param(&params, "Mouth:: Shape").is_some());
        assert!(get_param(&params, "Mouth:: Width").is_some());
    }

    #[test]
    fn thinking_motion_has_noticeable_live_ranges() {
        let samples = [0.0_f32, 2.0, 4.0, 6.0, 8.0, 10.0]
            .into_iter()
            .map(|time| create_motion_targets(RuntimeStatus::Thinking, time))
            .collect::<Vec<_>>();

        let head_samples = samples
            .iter()
            .filter_map(|params| get_param(params, "Head:: Yaw-Pitch").map(|param| param.x))
            .collect::<Vec<_>>();
        let body_samples = samples
            .iter()
            .filter_map(|params| get_param(params, "Body:: Yaw-Pitch").map(|param| param.x))
            .collect::<Vec<_>>();
        let blink_samples = samples
            .iter()
            .filter_map(|params| get_param(params, "Eye:: Left:: Blink").map(|param| param.x))
            .collect::<Vec<_>>();
        let breath_samples = samples
            .iter()
            .filter_map(|params| get_param(params, "Breath").map(|param| param.x))
            .collect::<Vec<_>>();
        let arm_right_samples = samples
            .iter()
            .filter_map(|params| get_param(params, "Arm:: Right:: Move").map(|param| param.x))
            .collect::<Vec<_>>();
        let tail_samples = samples
            .iter()
            .filter_map(|params| get_param(params, "Tail:: Move").map(|param| param.x))
            .collect::<Vec<_>>();

        let range =
            |s: &[f32]| s.iter().copied().fold(f32::NEG_INFINITY, f32::max) - s.iter().copied().fold(f32::INFINITY, f32::min);

        let head_range = range(&head_samples);
        let body_range = range(&body_samples);
        let blink_range = range(&blink_samples);
        let breath_range = range(&breath_samples);
        let arm_right_range = range(&arm_right_samples);
        let tail_range = range(&tail_samples);

        assert!(
            head_range > 0.15,
            "expected head range > 0.15, got {head_range}"
        );
        assert!(
            body_range > 0.08,
            "expected body range > 0.08, got {body_range}"
        );
        assert!(
            blink_range > 0.05,
            "expected blink range > 0.05, got {blink_range}"
        );
        assert!(
            breath_range > 0.10,
            "expected breath range > 0.10, got {breath_range}"
        );
        assert!(
            arm_right_range > 0.30,
            "expected arm_right range > 0.30, got {arm_right_range}"
        );
        assert!(
            tail_range > 0.15,
            "expected tail range > 0.15, got {tail_range}"
        );
    }

    #[test]
    fn thinking_phases_produce_distinct_poses() {
        let phase1 = create_motion_targets(RuntimeStatus::Thinking, 1.0);
        let phase2 = create_motion_targets(RuntimeStatus::Thinking, 5.0);
        let phase3 = create_motion_targets(RuntimeStatus::Thinking, 7.5);

        // Arm positions should differ significantly between phases
        let arm_r_1 = get_param(&phase1, "Arm:: Right:: Move").unwrap().x;
        let arm_r_3 = get_param(&phase3, "Arm:: Right:: Move").unwrap().x;
        assert!(
            (arm_r_3 - arm_r_1).abs() > 0.3,
            "expected arm phase difference > 0.3, got {}",
            (arm_r_3 - arm_r_1).abs()
        );

        // Head direction should flip between phase 1 and 2
        let head_1 = get_param(&phase1, "Head:: Yaw-Pitch").unwrap().x;
        let head_2 = get_param(&phase2, "Head:: Yaw-Pitch").unwrap().x;
        assert!(
            head_1 * head_2 < 0.0,
            "expected head to flip direction: phase1={head_1}, phase2={head_2}"
        );
    }

    #[test]
    fn idle_loop_is_seamless_at_start_and_end() {
        let first = create_motion_targets(RuntimeStatus::Idle, 0.0);
        let second = create_motion_targets(RuntimeStatus::Idle, IDLE_LOOP_SECONDS);

        assert_eq!(first, second);
    }

    #[test]
    fn idle_uses_full_parameter_set() {
        let params = create_motion_targets(RuntimeStatus::Idle, 0.0);

        assert!(get_param(&params, "Head:: Yaw-Pitch").is_some());
        assert!(get_param(&params, "Eye:: Left:: Blink").is_some());
        assert!(get_param(&params, "Eye:: Left:: Move").is_some());
        assert!(get_param(&params, "Breath").is_some());
        assert!(get_param(&params, "Tail:: Move").is_some());
        assert!(get_param(&params, "Mouth:: Shape").is_some());
    }

    #[test]
    fn idle_has_drowsy_eyes_during_sigh_phase() {
        // Phase 2 (sigh) is around t=5s: eyes should be more closed
        let alert_phase = create_motion_targets(RuntimeStatus::Idle, 9.0);
        let sigh_phase = create_motion_targets(RuntimeStatus::Idle, 5.0);

        let blink_alert = get_param(&alert_phase, "Eye:: Left:: Blink").unwrap().x;
        let blink_sigh = get_param(&sigh_phase, "Eye:: Left:: Blink").unwrap().x;

        // Higher blink value = more closed; sigh should be more closed
        assert!(
            blink_sigh > blink_alert,
            "expected sigh phase to have more closed eyes: sigh={blink_sigh}, alert={blink_alert}"
        );
    }

    #[test]
    fn idle_motion_has_visible_variation() {
        let samples = [0.0_f32, 3.0, 5.5, 8.0, 10.0, 13.0]
            .into_iter()
            .map(|time| create_motion_targets(RuntimeStatus::Idle, time))
            .collect::<Vec<_>>();

        let range = |name: &str, field: fn(&waifudex_mascot::MascotParamValue) -> f32| {
            let vals: Vec<f32> = samples
                .iter()
                .filter_map(|params| get_param(params, name).map(&field))
                .collect();
            vals.iter().copied().fold(f32::NEG_INFINITY, f32::max)
                - vals.iter().copied().fold(f32::INFINITY, f32::min)
        };

        let head_range = range("Head:: Yaw-Pitch", |p| p.x);
        let blink_range = range("Eye:: Left:: Blink", |p| p.x);
        let breath_range = range("Breath", |p| p.x);

        assert!(
            head_range > 0.10,
            "expected idle head range > 0.10, got {head_range}"
        );
        assert!(
            blink_range > 0.10,
            "expected idle blink range > 0.10, got {blink_range}"
        );
        assert!(
            breath_range > 0.10,
            "expected idle breath range > 0.10, got {breath_range}"
        );
    }
}
