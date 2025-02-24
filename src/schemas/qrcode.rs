use crate::schemas::{base::BaseSchema, JsonPosition};
use crate::utils::OpBuffer;
use printpdf::*;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonQrCodeSchema {
    name: String,
    position: JsonPosition,
    width: f32,
    height: f32,
    content: String,
}

pub struct QrCodeSchema {}

impl QrCodeSchema {
    pub fn from_json(json: JsonQrCodeSchema) -> Result<Self, String> {
        let base = BaseSchema::new(
            json.name,
            Mm(json.position.x),
            Mm(json.position.y),
            Mm(json.width),
            Mm(json.height),
        );
    }
}
