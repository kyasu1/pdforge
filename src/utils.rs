use printpdf::*;

#[derive(Debug, PartialEq, Clone, Default)]
pub struct OpBuffer {
    pub buffer: Vec<Vec<Op>>,
}

impl OpBuffer {
    pub fn insert(&mut self, page: usize, mut ops: Vec<Op>) {
        if page < self.buffer.len() {
            self.buffer[page].append(&mut ops)
        } else {
            self.buffer.push(ops);
        }
    }
}
/*
```Javascript
/**
 * If using dynamic font size, iteratively increment or decrement the
 * font size to fit the containing box.
 * Calculating space usage involves splitting lines where they exceed
 * the box width based on the proposed size.
 */
export const calculateDynamicFontSize = async ({
    textSchema,
    font,
    value,
    startingFontSize,
    _cache,
  }: {
    textSchema: TextSchema;
    font: Font;
    value: string;
    startingFontSize?: number | undefined;
    _cache: Map<any, any>;
  }) => {
    const {
      fontSize: schemaFontSize,
      dynamicFontSize: dynamicFontSizeSetting,
      characterSpacing: schemaCharacterSpacing,
      width: boxWidth,
      height: boxHeight,
      lineHeight = DEFAULT_LINE_HEIGHT,
    } = textSchema;
    const fontSize = startingFontSize || schemaFontSize || DEFAULT_FONT_SIZE;
    if (!dynamicFontSizeSetting) return fontSize;
    if (dynamicFontSizeSetting.max < dynamicFontSizeSetting.min) return fontSize;

    const characterSpacing = schemaCharacterSpacing ?? DEFAULT_CHARACTER_SPACING;
    const fontKitFont = await getFontKitFont(textSchema.fontName, font, _cache);
    const paragraphs = value.split('\n');

    let dynamicFontSize = fontSize;
    if (dynamicFontSize < dynamicFontSizeSetting.min) {
      dynamicFontSize = dynamicFontSizeSetting.min;
    } else if (dynamicFontSize > dynamicFontSizeSetting.max) {
      dynamicFontSize = dynamicFontSizeSetting.max;
    }
    const dynamicFontFit = dynamicFontSizeSetting.fit ?? DEFAULT_DYNAMIC_FIT;

    const calculateConstraints = (size: number) => {
      let totalWidthInMm = 0;
      let totalHeightInMm = 0;

      const boxWidthInPt = mm2pt(boxWidth);
      const firstLineTextHeight = heightOfFontAtSize(fontKitFont, size);
      const firstLineHeightInMm = pt2mm(firstLineTextHeight * lineHeight);
      const otherRowHeightInMm = pt2mm(size * lineHeight);

      paragraphs.forEach((paragraph, paraIndex) => {
        const lines = getSplittedLines(paragraph, {
          font: fontKitFont,
          fontSize: size,
          characterSpacing,
          boxWidthInPt,
        });
        lines.forEach((line, lineIndex) => {
          if (dynamicFontFit === DYNAMIC_FIT_VERTICAL) {
            // For vertical fit we want to consider the width of text lines where we detect a split
            const textWidth = widthOfTextAtSize(line, fontKitFont, size, characterSpacing);
            const textWidthInMm = pt2mm(textWidth);
            totalWidthInMm = Math.max(totalWidthInMm, textWidthInMm);
          }

          if (paraIndex + lineIndex === 0) {
            totalHeightInMm += firstLineHeightInMm;
          } else {
            totalHeightInMm += otherRowHeightInMm;
          }
        });
        if (dynamicFontFit === DYNAMIC_FIT_HORIZONTAL) {
          // For horizontal fit we want to consider the line's width 'unsplit'
          const textWidth = widthOfTextAtSize(paragraph, fontKitFont, size, characterSpacing);
          const textWidthInMm = pt2mm(textWidth);
          totalWidthInMm = Math.max(totalWidthInMm, textWidthInMm);
        }
      });

      return { totalWidthInMm, totalHeightInMm };
    };

    const shouldFontGrowToFit = (totalWidthInMm: number, totalHeightInMm: number) => {
      if (dynamicFontSize >= dynamicFontSizeSetting.max) {
        return false;
      }
      if (dynamicFontFit === DYNAMIC_FIT_HORIZONTAL) {
        return totalWidthInMm < boxWidth;
      }
      return totalHeightInMm < boxHeight;
    };

    const shouldFontShrinkToFit = (totalWidthInMm: number, totalHeightInMm: number) => {
      if (dynamicFontSize <= dynamicFontSizeSetting.min || dynamicFontSize <= 0) {
        return false;
      }
      return totalWidthInMm > boxWidth || totalHeightInMm > boxHeight;
    };

    let { totalWidthInMm, totalHeightInMm } = calculateConstraints(dynamicFontSize);

    // Attempt to increase the font size up to desired fit
    while (shouldFontGrowToFit(totalWidthInMm, totalHeightInMm)) {
      dynamicFontSize += FONT_SIZE_ADJUSTMENT;
      const { totalWidthInMm: newWidth, totalHeightInMm: newHeight } =
        calculateConstraints(dynamicFontSize);

      if (newHeight < boxHeight) {
        totalWidthInMm = newWidth;
        totalHeightInMm = newHeight;
      } else {
        dynamicFontSize -= FONT_SIZE_ADJUSTMENT;
        break;
      }
    }

    // Attempt to decrease the font size down to desired fit
    while (shouldFontShrinkToFit(totalWidthInMm, totalHeightInMm)) {
      dynamicFontSize -= FONT_SIZE_ADJUSTMENT;
      ({ totalWidthInMm, totalHeightInMm } = calculateConstraints(dynamicFontSize));
    }

    return dynamicFontSize;
  };
```
*/
