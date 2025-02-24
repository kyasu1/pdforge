use rust_pdfme::font::*;
use rust_pdfme::schemas::table::Table;
use rust_pdfme::schemas::text::Text;
use rust_pdfme::schemas::Schema;
use rust_pdfme::utils::OpBuffer;

use printpdf::*;

fn main() {
    // let template =
    //     rust_pdfme::schemas::Template::read_from_file("./templates/template-01.json").unwrap();
    let template =
        rust_pdfme::schemas::Template::read_from_file("./templates/table-test.json").unwrap();
    let mut doc = PdfDocument::new("TEST");

    let mut font_map = FontMap::default();

    let font_slice = include_bytes!(".././assets/fonts/NotoSerifJP-Regular.ttf");
    let parsed_font = ParsedFont::from_bytes(font_slice, 0).unwrap();
    let font_id = doc.add_font(&parsed_font);
    font_map.add_font(String::from("NotoSerif"), font_id.clone(), &parsed_font);

    let font_slice = include_bytes!(".././assets/fonts/NotoSansJP-Regular.ttf");
    let parsed_font = ParsedFont::from_bytes(font_slice, 0).unwrap();
    let font_id = doc.add_font(&parsed_font);
    font_map.add_font(String::from("NotoSans"), font_id.clone(), &parsed_font);

    let mut buffer = OpBuffer::default();

    let mut p = 0;
    let mut y: Option<Mm> = None;
    for (page_index, page) in template.schemas.iter().enumerate() {
        for schema in page {
            match schema {
                Schema::Text(obj) => {
                    let mut text = Text::new(obj, &font_map).unwrap();
                    let updated = text
                        .render(template.page_heihgt().into(), page_index, p, y, &mut buffer)
                        .unwrap();

                    if let Some((updated_p, updated_y)) = updated {
                        p = updated_p;
                        y = updated_y;
                    }
                }
                Schema::FlowingText(obj) => {
                    let mut text = Text::new(obj, &font_map).unwrap();
                    (p, y) = text
                        .draw2(template.page_heihgt().into(), p, y, &mut buffer)
                        .unwrap();
                }
                Schema::Table(obj) => {
                    let mut table = Table::new(obj, &font_map);
                    (p, y) = table
                        .render(template.page_heihgt(), p, y, &mut buffer)
                        .unwrap();
                }
            }

            println!("page {} next_y {:?}", p, y);
        }
    }
    //     let base = BaseSchema::new(String::from("test"), Kind::Dynamic, 10.0, 20.0, 160.0, 40.0);
    //
    //     let content = r#" 【ニューヨーク=佐藤璃子】21日の米株式市場でダウ工業株30種平均は続落し、前日比748ドル（2%）安の4万3428ドルで終えた。下げ幅は2024年12月中旬以来、約2カ月ぶりの大きさとなった。同日発表の米景気指標が想定以上に悪化し、リスク回避の株売りと安全資産とされる米国債への買いが広がった。米金利の低下で日米金利差の縮小が意識され、円買い・ドル売りも加速した。
    // 米S&Pグローバルが21日発表した2月の米国購買担当者景気指数（PMI、速報値）は総合で50.4と前月比2.3ポイント低下し、23年9月以来の低水準となった。悪化の要因となったのがサービス業の景況感で、49.7とおよそ2年ぶりに「不況」水準に落ち込み、市場予想の平均（52.8）を大きく割り込んだ。
    // 米ミシガン大学が同日発表した2月の消費者信頼感指数（確報値）も64.7と23年11月以来の低水準を記録した。いずれもトランプ米政権の関税政策などへの先行き懸念が強まっていることを示す内容で、堅調な米経済が悪化するとの不安につながった。
    // セブン&アイ・ホールディングス（HD）が傘下のスーパーなど非中核事業を束ねる中間持ち株会社の株式売却で、米投資ファンドのベインキャピタルに優先交渉権を与える見通しとなったことが22日、わかった。企業価値についてベインは7000億円以上を提示したとみられる。セブンはイトーヨーカ堂などの経営主体をファンドに移し、コンビニエンスストア事業に集中する。
    // セブンが22日までに臨時取締役会などを開いて決めた。今後、細部を詰め、3月末までの最終合意を目指す。売却対象となっている中間持ち株会社「ヨーク・ホールディングス（HD）」へのベインの出資額は今後、同社が提示した7000億円以上の企業価値をベースに具体的な出資比率を調整して決める。
    // セブンはヨークHDについて25年度に持ち分法適用会社とする方針だ。ヨークHDにはセブン創業家の伊藤家も一部出資することを検討している。
    // ヨーカ堂や食品スーパーのヨークベニマルなどを抱えるヨークHDの株式売却を巡っては、2024年11月末に締め切られた1次入札に、ベインや日本産業パートナーズ（JIP）のほか、KKR、住友商事、米フォートレス・インベストメント・グループなど少なくとも7社が応札した。その後、JIPやベイン、KKRの3社が1次入札を通過しセブン側に正式な買収提案をしていた。
    // セブン&アイ・ホールディングス（HD）が傘下のスーパーなど非中核事業を束ねる中間持ち株会社の株式売却で、米投資ファンドのベインキャピタルに優先交渉権を与える見通しとなったことが22日、わかった。企業価値についてベインは7000億円以上を提示したとみられる。セブンはイトーヨーカ堂などの経営主体をファンドに移し、コンビニエンスストア事業に集中する。
    // セブンが22日までに臨時取締役会などを開いて決めた。今後、細部を詰め、3月末までの最終合意を目指す。売却対象となっている中間持ち株会社「ヨーク・ホールディングス（HD）」へのベインの出資額は今後、同社が提示した7000億円以上の企業価値をベースに具体的な出資比率を調整して決める。
    // セブンはヨークHDについて25年度に持ち分法適用会社とする方針だ。ヨークHDにはセブン創業家の伊藤家も一部出資することを検討している。
    // ヨーカ堂や食品スーパーのヨークベニマルなどを抱えるヨークHDの株式売却を巡っては、2024年11月末に締め切られた1次入札に、ベインや日本産業パートナーズ（JIP）のほか、KKR、住友商事、米フォートレス・インベストメント・グループなど少なくとも7社が応札した。その後、JIPやベイン、KKRの3社が1次入札を通過しセブン側に正式な買収提案をしていた。
    // セブン&アイ・ホールディングス（HD）が傘下のスーパーなど非中核事業を束ねる中間持ち株会社の株式売却で、米投資ファンドのベインキャピタルに優先交渉権を与える見通しとなったことが22日、わかった。企業価値についてベインは7000億円以上を提示したとみられる。セブンはイトーヨーカ堂などの経営主体をファンドに移し、コンビニエンスストア事業に集中する。
    // セブンが22日までに臨時取締役会などを開いて決めた。今後、細部を詰め、3月末までの最終合意を目指す。売却対象となっている中間持ち株会社「ヨーク・ホールディングス（HD）」へのベインの出資額は今後、同社が提示した7000億円以上の企業価値をベースに具体的な出資比率を調整して決める。
    // セブンはヨークHDについて25年度に持ち分法適用会社とする方針だ。ヨークHDにはセブン創業家の伊藤家も一部出資することを検討している。
    // ヨーカ堂や食品スーパーのヨークベニマルなどを抱えるヨークHDの株式売却を巡っては、2024年11月末に締め切られた1次入札に、ベインや日本産業パートナーズ（JIP）のほか、KKR、住友商事、米フォートレス・インベストメント・グループなど少なくとも7社が応札した。その後、JIPやベイン、KKRの3社が1次入札を通過しセブン側に正式な買収提案をしていた。
    // "#;
    //     let dynamic = DynamicFontSize::new(Pt(16.0), Pt(24.0), DynamicFontSizeFit::Vertical);
    //
    //     let text_schema = TextSchema::new(
    //         base,
    //         content.to_string(),
    //         String::from("NotoSerif"),
    //         Pt(0.0),
    //         None,
    //         // FontSize::Dynamic(dynamic),
    //         FontSize::Fixed(Pt(16.0)),
    //     );
    //
    //     let header_schema = TextSchema::new(
    //         BaseSchema::new(String::from("header"), Kind::Fixed, 10.0, 10.0, 190.0, 5.0),
    //         String::from("RUST PDF ME !"),
    //         String::from("NotoSerif"),
    //         Pt(0.0),
    //         None,
    //         FontSize::Fixed(Pt(36.0)),
    //     );
    //
    //     let footer_schema = TextSchema::flowing(
    //         String::from("footer"),
    //         String::from("おしりだよーんｎ"),
    //         String::from("NotoSerif"),
    //         Pt(24.0),
    //         10.0,
    //         250.0,
    //         190.0,
    //         40.0,
    //     );

    // let mut text = Text::new(text_schema.clone(), &font_map).unwrap();
    // let (p, y) = text
    //     .draw2(page_height, current_page, None, &mut buffer)
    //     .unwrap();
    // let (p, y) = text.draw2(page_height, p, Some(y), &mut buffer).unwrap();
    //
    // let (p, y) = text.draw2(page_height, p, Some(y), &mut buffer).unwrap();
    //
    // let mut footer = Text::new(footer_schema, &font_map).unwrap();
    // footer.draw2(page_height, p, Some(y), &mut buffer).unwrap();
    //
    // println!("{} {:?}", p, y);
    //
    // current_page += 1;
    // let mut text = Text::new(text_schema, &font_map).unwrap();
    // text.draw(page_height.into(), current_page, &mut buffer)
    //     .unwrap();
    //
    // let mut header = Text::new(header_schema, &font_map).unwrap();
    // header.draw(page_height.into(), 0, &mut buffer).unwrap();

    // collect pages

    let mut pages: Vec<PdfPage> = Vec::new();
    for ops in buffer.buffer {
        let page = PdfPage::new(Mm(210.0), Mm(297.0), ops.to_vec());
        pages.push(page)
    }

    let bytes = doc.with_pages(pages).save(&PdfSaveOptions {
        optimize: false,
        subset_fonts: false,
    });
    std::fs::write("./simple.pdf", bytes).unwrap();
}
