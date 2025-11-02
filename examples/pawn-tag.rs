use std::collections::HashMap;
use std::fmt::Write;

// 文字コード範囲の定義
#[derive(Clone, Copy)]
struct CharRange {
    start: u32,
    end: u32,
}

const VALID_RANGES: &[CharRange] = &[
    CharRange {
        start: 0x0009,
        end: 0x0009,
    }, // タブ
    CharRange {
        start: 0x000A,
        end: 0x000A,
    }, // 改行
    CharRange {
        start: 0x000D,
        end: 0x000D,
    }, // 復帰
    CharRange {
        start: 0x0020,
        end: 0x007E,
    }, // 基本ラテン（制御文字を除く）
    CharRange {
        start: 0x3040,
        end: 0x309F,
    }, // 平仮名
    CharRange {
        start: 0x30A0,
        end: 0x30FF,
    }, // 片仮名
    CharRange {
        start: 0x4E00,
        end: 0x9FFF,
    }, // CJK統合漢字
    CharRange {
        start: 0xF900,
        end: 0xFAFF,
    }, // CJK互換漢字
    CharRange {
        start: 0x3000,
        end: 0x303F,
    }, // CJK用の記号及び分音記号
    CharRange {
        start: 0xFF00,
        end: 0xFF65,
    }, // 半角形・全角形（半角カナを除く）
    CharRange {
        start: 0xFFA0,
        end: 0xFFEF,
    }, // 半角形・全角形（続き）
    CharRange {
        start: 0x00A0,
        end: 0x00FF,
    }, // ラテン-1補助（制御文字を除く）
    CharRange {
        start: 0x2190,
        end: 0x21FF,
    }, // 矢印
    CharRange {
        start: 0x2000,
        end: 0x206F,
    }, // 一般句読点
    CharRange {
        start: 0x2500,
        end: 0x257F,
    }, // 罫線素片
    CharRange {
        start: 0x25A0,
        end: 0x25FF,
    }, // 幾何学模様
    CharRange {
        start: 0x0370,
        end: 0x03FF,
    }, // 基本ギリシャ
    CharRange {
        start: 0x0400,
        end: 0x04FF,
    }, // キリール
    CharRange {
        start: 0x2200,
        end: 0x22FF,
    }, // 数学記号
    CharRange {
        start: 0x2150,
        end: 0x218F,
    }, // 数字の形
    CharRange {
        start: 0x2460,
        end: 0x24FF,
    }, // 囲み英数字
    CharRange {
        start: 0x3200,
        end: 0x32FF,
    }, // 囲みCJK文字／月
    CharRange {
        start: 0x3300,
        end: 0x33FF,
    }, // CJK互換文字
];

// コードポイントが有効な範囲内にあるかチェック
fn is_valid_code_point(code_point: u32) -> bool {
    VALID_RANGES
        .iter()
        .any(|range| code_point >= range.start && code_point <= range.end)
}

// 文字列を処理し、無効な文字をHex文字列に置換
pub fn sanitize_string(input: &str) -> String {
    let mut output = String::new();
    for c in input.chars() {
        let code_point = c as u32;
        if is_valid_code_point(code_point) {
            output.push(c);
        } else {
            // 無効な文字を0xXXXX形式で置換（4桁のHex、大文字）
            write!(&mut output, "0x{:04X}", code_point).unwrap();
        }
    }
    output.replace("\\", "\\\\").replace("\"", "\\\"")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut pdforge = pdforge::PDForgeBuilder::new("PAWN_TAG_EXAMPLE".to_string())
        .add_font("NotoSerifJP", "./assets/fonts/NotoSerifJP-Regular.ttf")?
        .add_font("NotoSansJP", "./assets/fonts/NotoSansJP-Regular.ttf")?
        .add_font("NotoSans", "./assets/fonts/NotoSansJP-Regular.ttf")?
        .load_template("pawn_tag", "./templates/pawn-tag.json")?
        .build();

    let mut inputs: Vec<HashMap<&'static str, String>> = vec![];
    let mut input: HashMap<&'static str, String> = HashMap::new();

    input.insert("name", "田中 太郎".to_string());
    input.insert("pawnDate", "2024/08/30".to_string());
    input.insert("amount", "99,999,999".to_string());
    input.insert(
        "desc",
        sanitize_string(
            r#"ロレックス デイトナ "ステンレス製" ('4') \ ¥19,800 <> & シリアル番号: 123456789"#,
        )
        .replace("\\", "\\\\")
        .replace("\"", "\\\""),
    );
    input.insert("qrCode", "310001".to_string());
    input.insert("pawnSequence", "31-00001".to_string());

    inputs.push(input);

    let bytes: Vec<u8> = pdforge.render("pawn_tag", vec![inputs], None, None)?;

    std::fs::write("./examples/pdf/pawn-tag.pdf", bytes)?;

    println!("質札PDF generated successfully at ./examples/pdf/pawn-tag.pdf");
    Ok(())
}
