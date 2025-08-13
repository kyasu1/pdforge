use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut pdforge = pdforge::PDForgeBuilder::new("QUOTE_EXAMPLE".to_string())
        .add_font("NotoSansJP", "./assets/fonts/NotoSansJP-Regular.ttf")?
        .load_template("quote", "./templates/quote.json")?
        .build();

    let mut inputs: Vec<HashMap<&'static str, String>> = vec![];

    // Get current datetime for printing
    let print_datetime = format!("2025-08-13 14:30:00");

    let mut input: HashMap<&'static str, String> = HashMap::new();

    // Basic quote information
    input.insert("date", "2025-08-13".to_string());
    input.insert("version", "1".to_string());
    input.insert("customerName", "山田 太郎".to_string());
    input.insert("buyout_total", "45,250,000".to_string());
    input.insert("pawn_total", "37,200,000".to_string());
    input.insert("userName", "田中 花子".to_string());
    input.insert("qrCode", "QR".to_string()); // Placeholder for QR code content
    input.insert("dateTime", print_datetime.clone());

    inputs.push(input);

    // Sample items for the quote table (4 columns: No, 商品名, 買取査定, 質入査定)
    // Creating 99 items to test multi-page functionality
    let mut items = vec![];

    let sample_items = vec![
        ("ロレックス サブマリーナ", "800,000", "650,000"),
        ("プラチナ ネックレス 20g", "180,000", "150,000"),
        ("ダイヤモンド リング 1.5ct", "270,000", "200,000"),
        ("オメガ スピードマスター", "450,000", "380,000"),
        ("カルティエ タンク", "320,000", "260,000"),
        ("金 インゴット 100g", "750,000", "720,000"),
        ("プラチナ ブレスレット", "220,000", "180,000"),
        ("エルメス バーキン", "1,200,000", "1,000,000"),
        ("ルイ・ヴィトン バッグ", "150,000", "120,000"),
        ("シャネル ピアス", "95,000", "80,000"),
    ];

    for i in 1..=99 {
        let sample_index = (i - 1) % sample_items.len();
        let (name, buyout, pawn) = sample_items[sample_index];

        items.push(vec![
            format!("{:02}", i),
            format!("{} #{}", name, i),
            buyout.to_string(),
            pawn.to_string(),
        ]);
    }

    let mut table_data = HashMap::new();
    table_data.insert("items".to_string(), items);

    let bytes: Vec<u8> =
        pdforge.render_with_inputs_and_table_data("quote", vec![inputs], table_data)?;

    std::fs::write("./examples/pdf/quote.pdf", bytes.clone()).unwrap();

    println!("Quote PDF generated successfully at ./examples/pdf/quote.pdf");

    Ok(())
}
