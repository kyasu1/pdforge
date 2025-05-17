use printpdf::{Mm, Px, XObjectTransform};

#[derive(Debug, Clone)]
pub struct BaseSchema {
    pub(crate) name: String,
    pub(crate) x: Mm,
    pub(crate) y: Mm,
    pub(crate) width: Mm,
    pub(crate) height: Mm,
}

impl BaseSchema {
    pub fn new(name: String, x: Mm, y: Mm, width: Mm, height: Mm) -> Self {
        Self {
            name,
            x,
            y,
            width,
            height,
        }
    }

    pub fn get_matrix(&self, page_height: Mm, original_width: Option<Px>) -> XObjectTransform {
        let dpi: f32 = 300.0;
        let ratio: f32 = match original_width {
            Some(original_width) => dpi / 25.4 / (original_width.0 as f32),

            None => 1.0,
        };
        XObjectTransform {
            translate_x: Some(self.x.into()),
            translate_y: Some((page_height - self.y - self.height).into()),
            rotate: None,
            scale_x: Some(ratio * self.width.0),
            scale_y: Some(ratio * self.height.0),
            dpi: Some(dpi),
        }
    }
}
