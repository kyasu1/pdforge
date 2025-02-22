pub mod base;
pub mod text;

use printpdf::Mm;
use serde::de::Visitor;
use serde::{Deserialize, Deserializer};

#[derive(Debug, Clone)]
struct Size(Mm);

impl Size {
    pub fn new(v: f32) -> Self {
        Size(Mm(v))
    }
}
struct SizeVisitor;

impl<'de> Visitor<'de> for SizeVisitor {
    type Value = Size;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a floating point number representing size")
    }

    fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Size(Mm(v)))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Size(Mm(v as f32)))
    }
}

// deserializer.deserialize_f32(SizeVisitor)
// }

impl<'de> Deserialize<'de> for Size {
    fn deserialize<D>(deseriaplizer: D) -> Result<Size, D::Error>
    where
        D: Deserializer<'de>,
    {
        deseriaplizer.deserialize_f32(SizeVisitor)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub enum Schema {
    // Text(text::TextSchema),
    Text,
    FlowingText,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BasePdf {
    width: Size,
    height: Size,
    padding: Vec<Size>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Template {
    schemas: Vec<Schema>,
    base_pdf: BasePdf,
    version: String,
}
