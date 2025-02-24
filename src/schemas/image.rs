use crate::schemas::{base::BaseSchema, Error, JsonPosition};
use crate::utils::OpBuffer;
use printpdf::*;
use qrcode_generator::QrCodeEcc;
use serde::Deserialize;

pub struct Image {
    name: String,
    position: JsonPosition,
}
