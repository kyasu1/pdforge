use super::Size;
use derive_new::new;
use printpdf::Mm;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub enum Kind {
    Fixed(Size, Size),
    Flowing(Size),
}

#[derive(Debug, Clone, Deserialize)]
pub struct BaseSchema {
    name: String,
    type_: String,
    kind: Kind,
    x: Size,
    y: Size,
}

impl BaseSchema {
    pub fn fixed(name: String, type_: String, x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            name,
            type_,
            kind: Kind::Fixed(Size::new(width), Size::new(height)),
            x: Size::new(x),
            y: Size::new(y),
        }
    }

    pub fn flowing(name: String, type_: String, x: f32, y: f32, width: f32) -> Self {
        Self {
            name,
            type_,
            kind: Kind::Flowing(Size::new(width)),
            x: Size::new(x),
            y: Size::new(y),
        }
    }

    pub fn x(&self) -> Mm {
        self.x.0
    }

    pub fn y(&self) -> Mm {
        self.y.0
    }

    pub fn width(&self) -> Mm {
        match self.kind.clone() {
            Kind::Fixed(width, _) => width.0,
            Kind::Flowing(width) => width.0,
        }
    }

    pub fn height(&self) -> Option<Mm> {
        match self.kind.clone() {
            Kind::Fixed(_, height) => Some(height.0),
            Kind::Flowing(_) => None,
        }
    }
}
