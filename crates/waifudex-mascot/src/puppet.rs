use std::{collections::BTreeMap, path::Path};

use crate::{
    frame::{Color, FrameBuffer},
    MascotError, MascotParamValue, ParamInfo,
};

const AVAILABLE_PARAMS: [(&str, bool, [f32; 2], [f32; 2], [f32; 2]); 6] = [
    ("ParamAngleX", false, [-1.0, 0.0], [1.0, 0.0], [0.0, 0.0]),
    (
        "ParamBodyAngleX",
        false,
        [-1.0, 0.0],
        [1.0, 0.0],
        [0.0, 0.0],
    ),
    ("ParamEyeOpen", false, [0.0, 0.0], [1.0, 0.0], [1.0, 0.0]),
    ("ParamMouthOpenY", false, [0.0, 0.0], [1.0, 0.0], [0.0, 0.0]),
    ("ParamMouthSmile", false, [0.0, 0.0], [1.0, 0.0], [0.0, 0.0]),
    ("ParamBreath", false, [0.0, 0.0], [1.0, 0.0], [0.5, 0.0]),
];

#[derive(Debug)]
pub(crate) struct PuppetRenderer {
    available_params: Vec<ParamInfo>,
    params: BTreeMap<String, MascotParamValue>,
    dirty: bool,
}

impl PuppetRenderer {
    pub(crate) fn new(model_path: &Path) -> Result<Self, MascotError> {
        if !model_path.exists() {
            return Err(MascotError::ModelNotFound(model_path.to_path_buf()));
        }

        Ok(Self {
            available_params: AVAILABLE_PARAMS
                .iter()
                .map(|(name, is_vec2, min, max, defaults)| ParamInfo {
                    name: (*name).to_string(),
                    is_vec2: *is_vec2,
                    min: *min,
                    max: *max,
                    defaults: *defaults,
                })
                .collect(),
            params: BTreeMap::new(),
            dirty: true,
        })
    }

    pub(crate) fn available_params(&self) -> &[ParamInfo] {
        &self.available_params
    }

    pub(crate) fn set_param(&mut self, param: &MascotParamValue) -> bool {
        let previous = self.params.insert(param.name.clone(), param.clone());
        let changed = previous.as_ref() != Some(param);
        if changed {
            self.dirty = true;
        }
        changed
    }

    pub(crate) fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub(crate) fn render_if_needed(&mut self, frame: &mut FrameBuffer, _dt: f32) -> bool {
        if !self.dirty {
            return false;
        }

        self.dirty = false;
        frame.clear();

        let (width, height) = frame.dimensions();
        let body_angle = self.param_x("ParamBodyAngleX");
        let face_angle = self.param_x("ParamAngleX");
        let eye_open = self.param_y("ParamEyeOpen").clamp(0.0, 1.0);
        let mouth_open = self.param_y("ParamMouthOpenY").clamp(0.0, 1.0);
        let mouth_smile = self.param_y("ParamMouthSmile").clamp(0.0, 1.0);
        let breath = self.param_y("ParamBreath").clamp(0.0, 1.0);

        let width_f = width as f32;
        let height_f = height as f32;
        let body_center_x = width_f * (0.5 + body_angle * 0.12);
        let body_center_y = height_f * 0.73;
        let body_rx = width_f * (0.18 + breath * 0.02);
        let body_ry = height_f * (0.2 + breath * 0.015);

        let head_center_x = width_f * (0.5 + (face_angle + body_angle * 0.6) * 0.2);
        let head_center_y = height_f * (0.42 - breath * 0.015);
        let head_radius = width_f.min(height_f) * (0.205 + breath * 0.012);

        frame.draw_ellipse(
            body_center_x,
            body_center_y,
            body_rx,
            body_ry,
            Color {
                r: 66,
                g: 122,
                b: 188,
                a: 180,
            },
        );
        frame.draw_ellipse(
            head_center_x,
            head_center_y,
            head_radius,
            head_radius * 1.06,
            Color {
                r: 255,
                g: 226,
                b: 196,
                a: 242,
            },
        );

        let eye_offset_x = head_radius * 0.38;
        let eye_center_y = head_center_y - head_radius * 0.06;
        let eye_rx = head_radius * 0.1;
        let eye_ry = (head_radius * 0.16 * eye_open.max(0.14)).max(1.5);
        let eye_color = Color {
            r: 28,
            g: 43,
            b: 64,
            a: 255,
        };

        frame.draw_ellipse(
            head_center_x - eye_offset_x,
            eye_center_y,
            eye_rx,
            eye_ry,
            eye_color,
        );
        frame.draw_ellipse(
            head_center_x + eye_offset_x,
            eye_center_y,
            eye_rx,
            eye_ry,
            eye_color,
        );

        let mouth_center_y = head_center_y + head_radius * 0.34;
        let mouth_rx = head_radius * (0.18 + mouth_smile * 0.08);
        let mouth_ry = head_radius * (0.05 + mouth_open * 0.12);
        frame.draw_ellipse(
            head_center_x,
            mouth_center_y,
            mouth_rx,
            mouth_ry.max(1.5),
            Color {
                r: 177,
                g: 64,
                b: 88,
                a: 210,
            },
        );

        frame.increment_revision();
        true
    }

    fn param_x(&self, name: &str) -> f32 {
        self.params
            .get(name)
            .map(|value| value.x)
            .unwrap_or_default()
    }

    fn param_y(&self, name: &str) -> f32 {
        self.params
            .get(name)
            .map(|value| value.y)
            .unwrap_or_default()
    }
}
