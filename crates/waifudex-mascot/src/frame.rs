#[derive(Debug, Clone, Copy)]
pub(crate) struct Color {
    pub(crate) r: u8,
    pub(crate) g: u8,
    pub(crate) b: u8,
    pub(crate) a: u8,
}

#[derive(Debug)]
pub(crate) struct FrameBuffer {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
    revision: u64,
}

impl FrameBuffer {
    pub(crate) fn new(width: u32, height: u32) -> Self {
        let width = width.max(1);
        let height = height.max(1);
        Self {
            width,
            height,
            rgba: vec![0; (width as usize) * (height as usize) * 4],
            revision: 0,
        }
    }

    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        let next_width = width.max(1);
        let next_height = height.max(1);
        if self.width == next_width && self.height == next_height {
            return;
        }

        self.width = next_width;
        self.height = next_height;
        self.rgba
            .resize((self.width as usize) * (self.height as usize) * 4, 0);
    }

    pub(crate) fn clear(&mut self) {
        self.rgba.fill(0);
    }

    pub(crate) fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub(crate) fn pixels(&self) -> &[u8] {
        &self.rgba
    }

    pub(crate) fn increment_revision(&mut self) {
        self.revision = self.revision.saturating_add(1);
    }

    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }

    pub(crate) fn draw_ellipse(
        &mut self,
        center_x: f32,
        center_y: f32,
        radius_x: f32,
        radius_y: f32,
        color: Color,
    ) {
        if radius_x <= 0.0 || radius_y <= 0.0 {
            return;
        }

        let min_x = ((center_x - radius_x).floor() as i32).max(0);
        let max_x = ((center_x + radius_x).ceil() as i32).min(self.width as i32 - 1);
        let min_y = ((center_y - radius_y).floor() as i32).max(0);
        let max_y = ((center_y + radius_y).ceil() as i32).min(self.height as i32 - 1);

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let dx = ((x as f32 + 0.5) - center_x) / radius_x;
                let dy = ((y as f32 + 0.5) - center_y) / radius_y;
                if dx * dx + dy * dy <= 1.0 {
                    self.blend_pixel(x as u32, y as u32, color);
                }
            }
        }
    }

    fn blend_pixel(&mut self, x: u32, y: u32, color: Color) {
        let index = ((y * self.width + x) * 4) as usize;
        let alpha = color.a as f32 / 255.0;
        let inv_alpha = 1.0 - alpha;

        let current_r = self.rgba[index] as f32;
        let current_g = self.rgba[index + 1] as f32;
        let current_b = self.rgba[index + 2] as f32;
        let current_a = self.rgba[index + 3] as f32 / 255.0;

        let next_a = alpha + current_a * inv_alpha;
        self.rgba[index] = (color.r as f32 * alpha + current_r * inv_alpha).round() as u8;
        self.rgba[index + 1] = (color.g as f32 * alpha + current_g * inv_alpha).round() as u8;
        self.rgba[index + 2] = (color.b as f32 * alpha + current_b * inv_alpha).round() as u8;
        self.rgba[index + 3] = (next_a * 255.0).round() as u8;
    }
}
