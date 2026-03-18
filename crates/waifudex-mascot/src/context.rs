#[derive(Debug, Clone, Copy)]
pub(crate) struct RendererContext {
    width: u32,
    height: u32,
}

impl RendererContext {
    pub(crate) fn new(width: u32, height: u32) -> Self {
        Self {
            width: width.max(1),
            height: height.max(1),
        }
    }

    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        self.width = width.max(1);
        self.height = height.max(1);
    }

    pub(crate) fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}
