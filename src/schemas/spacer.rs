use super::{base::BaseSchema, BasePdf, Error, HasBaseSchema};
use printpdf::Mm;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct JsonSpacerSchema {
    pub height: f32,
}

#[derive(Debug, Clone)]
pub struct Spacer {
    base: BaseSchema,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct FlowCursor {
    pub page: usize,
    pub y: Option<Mm>,
}

impl FlowCursor {
    pub fn new(page: usize) -> Self {
        Self { page, y: None }
    }
}

impl Spacer {
    pub fn from_json(json: JsonSpacerSchema) -> Result<Self, Error> {
        if json.height < 0.0 {
            return Err(Error::InvalidSpacerHeight {
                height: json.height,
            });
        }

        Ok(Self {
            base: BaseSchema::new(
                "spacer".to_string(),
                Mm(0.0),
                Mm(0.0),
                Mm(0.0),
                Mm(json.height),
            ),
        })
    }

    pub(crate) fn advance(&self, base_pdf: &BasePdf, cursor: &mut FlowCursor) {
        let y = cursor.y.unwrap_or(base_pdf.padding.top);
        let bottom = base_pdf.height - base_pdf.padding.bottom;

        if y + self.base.height <= bottom {
            cursor.y = Some(y + self.base.height);
        } else {
            cursor.page += 1;
            cursor.y = Some(base_pdf.padding.top);
        }
    }
}

impl HasBaseSchema for Spacer {
    fn base(&self) -> &BaseSchema {
        &self.base
    }

    fn base_mut(&mut self) -> &mut BaseSchema {
        &mut self.base
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schemas::Frame;

    fn base_pdf() -> BasePdf {
        BasePdf {
            width: Mm(210.0),
            height: Mm(297.0),
            padding: Frame {
                top: Mm(10.0),
                right: Mm(10.0),
                bottom: Mm(10.0),
                left: Mm(10.0),
            },
            static_schema: vec![],
        }
    }

    fn spacer(height: f32) -> Spacer {
        Spacer::from_json(JsonSpacerSchema { height }).unwrap()
    }

    #[test]
    fn first_spacer_starts_at_top_padding() {
        let mut cursor = FlowCursor::new(0);

        spacer(5.0).advance(&base_pdf(), &mut cursor);

        assert_eq!(
            cursor,
            FlowCursor {
                page: 0,
                y: Some(Mm(15.0))
            }
        );
    }

    #[test]
    fn zero_height_spacer_does_not_move_started_cursor() {
        let mut cursor = FlowCursor {
            page: 2,
            y: Some(Mm(40.0)),
        };

        spacer(0.0).advance(&base_pdf(), &mut cursor);

        assert_eq!(
            cursor,
            FlowCursor {
                page: 2,
                y: Some(Mm(40.0))
            }
        );
    }

    #[test]
    fn spacer_that_fits_advances_on_current_page() {
        let mut cursor = FlowCursor {
            page: 1,
            y: Some(Mm(40.0)),
        };

        spacer(5.0).advance(&base_pdf(), &mut cursor);

        assert_eq!(
            cursor,
            FlowCursor {
                page: 1,
                y: Some(Mm(45.0))
            }
        );
    }

    #[test]
    fn spacer_that_overflows_discards_excess_and_moves_to_next_page_top() {
        let mut cursor = FlowCursor {
            page: 1,
            y: Some(Mm(285.0)),
        };

        spacer(5.0).advance(&base_pdf(), &mut cursor);

        assert_eq!(
            cursor,
            FlowCursor {
                page: 2,
                y: Some(Mm(10.0))
            }
        );
    }
}
