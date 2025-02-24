use printpdf::Mm;

#[derive(Debug, Clone)]
pub enum Kind {
    Fixed,
    Dynamic,
}

#[derive(Debug, Clone)]
pub struct BaseSchema {
    name: String,
    kind: Kind,
    x: Mm,
    y: Mm,
    width: Mm,
    height: Mm,
}

impl BaseSchema {
    pub fn new(name: String, kind: Kind, x: Mm, y: Mm, width: Mm, height: Mm) -> Self {
        Self {
            name,
            kind,
            x,
            y,
            width,
            height,
        }
    }

    pub fn kind(&self) -> Kind {
        self.kind.clone()
    }

    pub fn x(&self) -> Mm {
        self.x
    }

    pub fn y(&self) -> Mm {
        self.y
    }

    pub fn width(&self) -> Mm {
        self.width
    }

    pub fn height(&self) -> Option<Mm> {
        match self.kind.clone() {
            Kind::Fixed => Some(self.height),
            Kind::Dynamic => None,
        }
    }

    pub fn set_y(&mut self, y: Mm) {
        self.y = y;
    }
}
