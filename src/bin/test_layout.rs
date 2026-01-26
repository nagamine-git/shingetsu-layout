use kana_layout_optimizer::layout::{Layout, HIRAGANA_FREQ_DEFAULT, cols_for_row, NUM_LAYERS, ROWS};

fn main() {
    println!("=== 新月配列 v2.0 テスト（4層版）===\n");

    // 初期配列を表示
    let layout = Layout::improved_custom();

    println!("初期配列（4層）:\n");

    for layer in 0..NUM_LAYERS {
        let layer_name = match layer {
            0 => "Layer 0 (No Shift)",
            1 => "Layer 1 (☆ shift)",
            2 => "Layer 2 (★ shift)",
            3 => "Layer 3 (◆ shift)",
            _ => "Unknown",
        };

        println!("## {}", layer_name);
        for row in 0..ROWS {
            print!("  ");
            let cols = cols_for_row(row);
            for col in 0..cols {
                let c = &layout.layers[layer][row][col];
                // 1文字なら1文字分、2文字なら2文字分表示
                if c.chars().count() == 1 {
                    print!("{:3}", c);
                } else {
                    print!("{:4}", c);
                }
            }
            println!();
        }
        println!();
    }

    // 統計情報
    let mut gram1_count = 0;
    let mut gram2_count = 0;
    let mut blank_count = 0;
    let mut fixed_count = 0;

    let fixed_chars = ["★", "☆", "◆", "、", "。", "ー", "・", ";"];

    for layer in 0..NUM_LAYERS {
        for row in 0..ROWS {
            let cols = cols_for_row(row);
            for col in 0..cols {
                let c = &layout.layers[layer][row][col];
                if c == "　" {
                    blank_count += 1;
                } else if fixed_chars.contains(&c.as_str()) {
                    fixed_count += 1;
                } else if c.chars().count() == 1 {
                    gram1_count += 1;
                } else if c.chars().count() == 2 {
                    gram2_count += 1;
                }
            }
        }
    }

    println!("=== 統計 ===");
    println!("  1gram文字: {}", gram1_count);
    println!("  2gram文字（拗音）: {}", gram2_count);
    println!("  配置文字合計: {}", gram1_count + gram2_count);
    println!("  空白: {}", blank_count);
    println!("  固定文字: {} (★,☆,◆,、,。,ー,・,;)", fixed_count);
    let total = gram1_count + gram2_count + blank_count + fixed_count;
    println!("  総計: {} (= 4層 x 31 = 124)", total);

    println!("\n=== HIRAGANA_FREQ_DEFAULT ===");
    println!("  文字数: {}", HIRAGANA_FREQ_DEFAULT.len());

    // 検証
    let result = layout.validate(HIRAGANA_FREQ_DEFAULT);
    result.print_report();
}
