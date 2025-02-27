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

    pub fn get_matrix(&self, page_height: Mm, box_width: Option<Px>) -> XObjectTransform {
        let ratio: f32 = match box_width {
            Some(box_width) => 300.0 / (box_width.0 as f32) / 25.4,

            None => 1.0,
        };
        XObjectTransform {
            translate_x: Some(self.x.into()),
            translate_y: Some((page_height - self.y).into()),
            rotate: None,
            scale_x: Some(ratio * self.width.0),
            scale_y: Some(ratio * self.height.0),
            dpi: None,
        }
    }
}
