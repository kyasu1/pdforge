use super::{base::BaseSchema, BasePdf, Schema, SchemaTrait};
use crate::schemas::{Error, JsonPosition};
use crate::utils::OpBuffer;
use printpdf::{CurTransMat, Mm, Op, PdfDocument};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonGroupSchema {
    name: String,
    position: JsonPosition,
    width: f32,
    height: f32,
    rotate: Option<f32>,
    schemas: Vec<crate::schemas::JsonSchema>,
}

#[derive(Debug, Clone)]
pub struct Group {
    base: BaseSchema,
    rotate: Option<f32>,
    schemas: Vec<Schema>,
}

impl Group {
    pub fn from_json(
        json: JsonGroupSchema,
        font_map: &crate::font::FontMap,
    ) -> Result<Self, Error> {
        use crate::schemas::JsonSchema;

        let base = BaseSchema::new(
            json.name,
            Mm(json.position.x),
            Mm(json.position.y),
            Mm(json.width),
            Mm(json.height),
        );

        let mut schemas = Vec::new();
        for json_schema in json.schemas {
            let schema = match json_schema {
                JsonSchema::Text(schema) => {
                    Schema::Text(crate::schemas::text::Text::from_json(schema, font_map)?)
                }
                JsonSchema::DynamicText(schema) => Schema::DynamicText(
                    crate::schemas::dynamic_text::DynamicText::from_json(schema, font_map)?,
                ),
                JsonSchema::Table(schema) => {
                    Schema::Table(crate::schemas::table::Table::from_json(schema, font_map)?)
                }
                JsonSchema::QrCode(schema) => schema.into(),
                JsonSchema::Image(schema) => schema.try_into()?,
                JsonSchema::Svg(schema) => schema.try_into()?,
                JsonSchema::Rectangle(schema) => schema.try_into()?,
                _ => {
                    return Err(Error::Whatever {
                        message: "Group schema does not support this type".to_string(),
                        source: None,
                    })
                }
            };
            schemas.push(schema);
        }

        Ok(Group {
            base,
            rotate: json.rotate,
            schemas,
        })
    }

    pub fn get_base(self) -> BaseSchema {
        self.base
    }

    pub fn render(
        &mut self,
        base_pdf: &BasePdf,
        doc: &mut PdfDocument,
        page: usize,
        buffer: &mut OpBuffer,
    ) -> Result<(), Error> {
        let matrix_values = super::pdf_utils::calculate_transform_matrix_with_center_pivot(
            self.base.x,
            base_pdf.height - self.base.y - self.base.height,
            self.base.width,
            self.base.height,
            self.rotate,
        );

        let matrix: Op = Op::SetTransformationMatrix {
            matrix: CurTransMat::Raw(matrix_values),
        };

        // 各スキーマをレンダリング

        buffer.insert(page, vec![Op::SaveGraphicsState, matrix]);
        for schema in &mut self.schemas {
            schema.render(self.base.height, doc, page, buffer)?;
        }
        buffer.insert(page, vec![Op::RestoreGraphicsState]);

        Ok(())
    }

    pub fn set_x(&mut self, x: Mm) {
        self.base.x = x;
    }

    pub fn set_y(&mut self, y: Mm) {
        self.base.y = y;
    }

    pub fn set_height(&mut self, height: Mm) {
        self.base.height = height;
    }

    pub fn get_width(&self) -> Mm {
        self.base.width
    }

    pub fn get_height(&self) -> Mm {
        self.base.height
    }
}

impl TryFrom<JsonGroupSchema> for Schema {
    type Error = Error;

    fn try_from(json: JsonGroupSchema) -> Result<Self, Self::Error> {
        // フォントマップが必要なため、この実装は制限されます
        // 実際の使用では、from_jsonメソッドを使用してください
        todo!("Use Group::from_json instead")
    }
}
