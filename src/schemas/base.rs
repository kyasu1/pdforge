use printpdf::Mm;

#[derive(Debug, Clone)]
pub struct BaseSchema {
    name: String,
    x: Mm,
    y: Mm,
    width: Mm,
    height: Mm,
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

    pub fn x(&self) -> Mm {
        self.x
    }

    pub fn y(&self) -> Mm {
        self.y
    }

    pub fn width(&self) -> Mm {
        self.width
    }

    pub fn height(&self) -> Mm {
        self.height
    }

    pub fn set_y(&mut self, y: Mm) {
        self.y = y;
    }
}
