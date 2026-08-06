use printpdf::{Mm, Px, XObjectRotation, XObjectTransform};

pub(crate) const XOBJECT_DPI: f32 = 300.0;

/// Builds a center-pivot rotation after accounting for the XObject's manual scale.
///
/// `printpdf` applies image/SVG auto-scaling and `scale_x`/`scale_y` before the
/// pivot, so an intrinsic `width / 2, height / 2` pivot would shift scaled content.
pub(crate) fn rotation_for_xobject(
    angle_ccw_degrees: f32,
    width: Px,
    height: Px,
    transform: &XObjectTransform,
) -> XObjectRotation {
    let scale_x = transform.scale_x.unwrap_or(1.0);
    let scale_y = transform.scale_y.unwrap_or(1.0);

    XObjectRotation {
        angle_ccw_degrees,
        rotation_center_x: Px(((width.0 as f32 * scale_x) / 2.0).round().max(0.0) as usize),
        rotation_center_y: Px(((height.0 as f32 * scale_y) / 2.0).round().max(0.0) as usize),
    }
}

#[derive(Debug, Clone)]
pub struct BaseSchema {
    pub name: String,
    pub x: Mm,
    pub y: Mm,
    pub width: Mm,
    pub height: Mm,
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
        self.get_matrix_with_rotation(page_height, original_width, None)
    }

    pub fn get_matrix_with_rotation(
        &self,
        page_height: Mm,
        original_width: Option<Px>,
        rotation: Option<XObjectRotation>,
    ) -> XObjectTransform {
        let dpi = XOBJECT_DPI;
        let ratio: f32 = match original_width {
            Some(original_width) => dpi / 25.4 / (original_width.0 as f32),

            None => 1.0,
        };
        XObjectTransform {
            translate_x: Some(self.x.into()),
            translate_y: Some((page_height - self.y - self.height).into()),
            rotate: rotation,
            scale_x: Some(ratio * self.width.0),
            scale_y: Some(ratio * self.height.0),
            dpi: Some(dpi),
            no_auto_scale: false,
        }
    }
}
