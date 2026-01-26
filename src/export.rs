//! 配列エクスポートモジュール
//!
//! 生成された配列を複数の形式でエクスポートする。

use crate::layout::{Layout, cols_for_row, COLS, ROWS};
use std::collections::HashMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// QWERTYキーボードの物理キー配列
const QWERTY_KEYS: [[&str; 10]; 3] = [
    ["q", "w", "e", "r", "t", "y", "u", "i", "o", "p"],
    ["a", "s", "d", "f", "g", "h", "j", "k", "l", ";"],
    ["z", "x", "c", "v", "b", "n", "m", ",", ".", "/"],
];

/// Colemakキーボードの物理キー配列
const COLEMAK_KEYS: [[&str; 10]; 3] = [
    ["q", "w", "f", "p", "g", "j", "l", "u", "y", ";"],
    ["a", "r", "s", "t", "d", "h", "n", "e", "i", "o"],
    ["z", "x", "c", "v", "b", "k", "m", ",", ".", "/"],
];

/// ひらがな→ローマ字変換テーブル
fn kana_to_romaji() -> HashMap<char, &'static str> {
    let mut map = HashMap::new();
    // 清音
    map.insert('あ', "a"); map.insert('い', "i"); map.insert('う', "u");
    map.insert('え', "e"); map.insert('お', "o");
    map.insert('か', "ka"); map.insert('き', "ki"); map.insert('く', "ku");
    map.insert('け', "ke"); map.insert('こ', "ko");
    map.insert('さ', "sa"); map.insert('し', "si"); map.insert('す', "su");
    map.insert('せ', "se"); map.insert('そ', "so");
    map.insert('た', "ta"); map.insert('ち', "ti"); map.insert('つ', "tu");
    map.insert('て', "te"); map.insert('と', "to");
    map.insert('な', "na"); map.insert('に', "ni"); map.insert('ぬ', "nu");
    map.insert('ね', "ne"); map.insert('の', "no");
    map.insert('は', "ha"); map.insert('ひ', "hi"); map.insert('ふ', "fu");
    map.insert('へ', "he"); map.insert('ほ', "ho");
    map.insert('ま', "ma"); map.insert('み', "mi"); map.insert('む', "mu");
    map.insert('め', "me"); map.insert('も', "mo");
    map.insert('や', "ya"); map.insert('ゆ', "yu"); map.insert('よ', "yo");
    map.insert('ら', "ra"); map.insert('り', "ri"); map.insert('る', "ru");
    map.insert('れ', "re"); map.insert('ろ', "ro");
    map.insert('わ', "wa"); map.insert('を', "wo"); map.insert('ん', "nn");
    // 濁音
    map.insert('が', "ga"); map.insert('ぎ', "gi"); map.insert('ぐ', "gu");
    map.insert('げ', "ge"); map.insert('ご', "go");
    map.insert('ざ', "za"); map.insert('じ', "zi"); map.insert('ず', "zu");
    map.insert('ぜ', "ze"); map.insert('ぞ', "zo");
    map.insert('だ', "da"); map.insert('ぢ', "di"); map.insert('づ', "du");
    map.insert('で', "de"); map.insert('ど', "do");
    map.insert('ば', "ba"); map.insert('び', "bi"); map.insert('ぶ', "bu");
    map.insert('べ', "be"); map.insert('ぼ', "bo");
    // 半濁音
    map.insert('ぱ', "pa"); map.insert('ぴ', "pi"); map.insert('ぷ', "pu");
    map.insert('ぺ', "pe"); map.insert('ぽ', "po");
    // 小書き
    map.insert('ぁ', "xa"); map.insert('ぃ', "xi"); map.insert('ぅ', "xu");
    map.insert('ぇ', "xe"); map.insert('ぉ', "xo");
    map.insert('ゃ', "xya"); map.insert('ゅ', "xyu"); map.insert('ょ', "xyo");
    map.insert('っ', "xtu");
    // 特殊
    map.insert('ー', "-");
    map.insert('ゔ', "vu");
    map.insert('ヴ', "vu");
    map
}

/// Karabiner用のキーコード変換
fn romaji_to_keycode(romaji: &str) -> Vec<serde_json::Value> {
    romaji.chars().map(|c| {
        let key_code = match c {
            'a'..='z' => c.to_string(),
            '-' => "hyphen".to_string(),
            ';' => "semicolon".to_string(),
            ',' => "comma".to_string(),
            '.' => "period".to_string(),
            '/' => "slash".to_string(),
            _ => c.to_string(),
        };
        serde_json::json!({"key_code": key_code})
    }).collect()
}

/// 全形式でエクスポート
pub fn export_all(layout: &Layout, base_name: &str) {
    let base_path = Path::new(base_name);
    let stem = base_path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("layout");
    let parent = base_path.parent().unwrap_or(Path::new("."));

    // タイムスタンプを生成（UNIXエポックからの秒数）
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let stem_with_timestamp = format!("{}_{}", stem, timestamp);

    // 日時フォルダを作成（results/YYYY-MM-DD_HH-MM-SS/）
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    // 日本時間（UTC+9）に変換
    let jst_offset = 9 * 3600;
    let jst_timestamp = now + jst_offset;
    let days_since_epoch = jst_timestamp / 86400;
    let seconds_in_day = jst_timestamp % 86400;

    // 1970-01-01からの日数を年月日に変換
    let (year, month, day) = days_to_date(days_since_epoch as i32);

    // 秒数を時分秒に変換
    let hour = seconds_in_day / 3600;
    let minute = (seconds_in_day % 3600) / 60;
    let second = seconds_in_day % 60;

    let datetime_str = format!("{:04}-{:02}-{:02}_{:02}-{:02}-{:02}",
        year, month, day, hour, minute, second);

    // 出力ディレクトリを作成: results/YYYY-MM-DD_HH-MM-SS/
    let output_dir = parent.join("results").join(&datetime_str);
    std::fs::create_dir_all(&output_dir).unwrap_or_else(|e| {
        eprintln!("出力ディレクトリ作成エラー: {} ({})", e, output_dir.display());
    });

    println!("出力先: {}/", output_dir.display());

    // 1. JSON (best_layout.json形式)
    export_json(layout, &output_dir.join(format!("{}.json", stem_with_timestamp)));

    // 2. keyboard_analyzer用JSON
    export_analyzer_json(layout, &output_dir.join(format!("{}_analyzer.json", stem_with_timestamp)));

    // 3. hazkey TSV (QWERTY)
    export_tsv(layout, &output_dir.join(format!("{}-ansi.tsv", stem_with_timestamp)), false);

    // 4. hazkey TSV (Colemak)
    export_tsv(layout, &output_dir.join(format!("{}-ansi-colemak.tsv", stem_with_timestamp)), true);

    // 5. Karabiner JSON
    export_karabiner(layout, &output_dir.join(format!("{}-karabiner.json", stem_with_timestamp)));
}

/// UNIXエポックからの日数を年月日に変換
/// 簡易実装（グレゴリオ暦、閏年対応）
fn days_to_date(days: i32) -> (i32, i32, i32) {
    let mut year = 1970;
    let mut remaining_days = days;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    let days_in_months = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1;
    for &days_in_month in &days_in_months {
        if remaining_days < days_in_month {
            break;
        }
        remaining_days -= days_in_month;
        month += 1;
    }

    let day = remaining_days + 1;
    (year, month, day)
}

/// 閏年判定
fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// JSON形式でエクスポート（既存形式）
pub fn export_json(layout: &Layout, path: &Path) {
    let json = serde_json::json!({
        "name": "新月配列 v2.0 (Shingetsu Layout)",
        "fitness": layout.fitness,
        "scores": layout.scores,
        "layers": {
            "no_shift": layout.layers[0],
            "shift_star": layout.layers[1],  // ☆シフト
            "shift_star2": layout.layers[2], // ★シフト
            "shift_diamond": layout.layers[3], // ◆シフト
        }
    });

    match std::fs::write(path, serde_json::to_string_pretty(&json).unwrap()) {
        Ok(_) => println!("  JSON: {:?}", path),
        Err(e) => eprintln!("  JSON保存エラー: {}", e),
    }
}

/// keyboard_analyzer用JSON形式でエクスポート
pub fn export_analyzer_json(layout: &Layout, path: &Path) {

    // keysセクションを構築
    let mut keys = vec![];

    // 数字行
    keys.push(serde_json::json!([
        {"id": "`", "legend": ["`", "~"], "size": 1, "finger": 0},
        {"id": "1", "legend": ["1", "!"], "size": 1, "finger": 0},
        {"id": "2", "legend": ["2", "@"], "size": 1, "finger": 1},
        {"id": "3", "legend": ["3", "#"], "size": 1, "finger": 2},
        {"id": "4", "legend": ["4", "$"], "size": 1, "finger": 3},
        {"id": "5", "legend": ["5", "%"], "size": 1, "finger": 3},
        {"id": "6", "legend": ["6", "^"], "size": 1, "finger": 6},
        {"id": "7", "legend": ["7", "&"], "size": 1, "finger": 6},
        {"id": "8", "legend": ["8", "*"], "size": 1, "finger": 7},
        {"id": "9", "legend": ["9", "("], "size": 1, "finger": 8},
        {"id": "0", "legend": ["0", ")"], "size": 1, "finger": 9},
        {"id": "-", "legend": ["-", "_"], "size": 1, "finger": 9},
        {"id": "=", "legend": ["=", "+"], "size": 1, "finger": 9},
        {"id": "bs", "legend": ["BS"], "size": 2, "finger": 9}
    ]));

    // 上段 (10 cols)
    let mut top_row = vec![serde_json::json!({"id": "tab", "legend": ["Tab"], "size": 1.5, "finger": 0})];
    for col in 0..10 {
        let l0 = &layout.layers[0][0][col];
        let l1 = &layout.layers[1][0][col];
        let l2 = &layout.layers[2][0][col];
        let l3 = &layout.layers[3][0][col];
        let key_id = QWERTY_KEYS[0][col];
        let finger = if col < 5 { col.min(3) } else { 6 + (col - 5).min(3) };
        top_row.push(serde_json::json!({
            "id": key_id,
            "legend": [l0.clone(), l1.clone(), l2.clone(), l3.clone()],
            "size": 1,
            "finger": finger
        }));
    }
    top_row.push(serde_json::json!({"id": "[", "legend": ["「", "「", "「", "「"], "size": 1, "finger": 9}));
    top_row.push(serde_json::json!({"id": "]", "legend": ["」", "」", "」", "」"], "size": 1, "finger": 9}));
    top_row.push(serde_json::json!({"id": "\\", "legend": ["\\", "|"], "size": 1.5, "finger": 9}));
    keys.push(serde_json::Value::Array(top_row));

    // 中段 (11 cols - includes the extra ' key)
    let mut mid_row = vec![serde_json::json!({"id": "caps", "legend": ["Caps Lock"], "size": 1.75, "finger": 0})];
    for col in 0..10 {
        let l0 = &layout.layers[0][1][col];
        let l1 = &layout.layers[1][1][col];
        let l2 = &layout.layers[2][1][col];
        let l3 = &layout.layers[3][1][col];
        let key_id = QWERTY_KEYS[1][col];
        let finger = if col < 5 { col.min(3) } else { 6 + (col - 5).min(3) };
        let is_home = col >= 3 && col <= 6 || col == 0 || col == 9;
        let mut key_obj = serde_json::json!({
            "id": key_id,
            "legend": [l0.clone(), l1.clone(), l2.clone(), l3.clone()],
            "size": 1,
            "finger": finger
        });
        if is_home {
            key_obj["home"] = serde_json::json!(true);
        }
        mid_row.push(key_obj);
    }
    // col 10 (extra key for ー/・/;)
    let l0_extra = &layout.layers[0][1][10];
    let l1_extra = &layout.layers[1][1][10];
    let l2_extra = &layout.layers[2][1][10];
    let l3_extra = &layout.layers[3][1][10];
    mid_row.push(serde_json::json!({
        "id": "'",
        "legend": [l0_extra.clone(), l1_extra.clone(), l2_extra.clone(), l3_extra.clone()],
        "size": 1,
        "finger": 9
    }));
    mid_row.push(serde_json::json!({"id": "enter", "legend": ["Enter"], "size": 2.25, "finger": 9}));
    keys.push(serde_json::Value::Array(mid_row));

    // 下段 (10 cols)
    let mut bot_row = vec![serde_json::json!({"id": "shift", "legend": ["Shift"], "size": 2.25, "finger": 0})];
    for col in 0..10 {
        let l0 = &layout.layers[0][2][col];
        let l1 = &layout.layers[1][2][col];
        let l2 = &layout.layers[2][2][col];
        let l3 = &layout.layers[3][2][col];
        let key_id = QWERTY_KEYS[2][col];
        let finger = if col < 5 { col.min(3) } else { 6 + (col - 5).min(3) };
        bot_row.push(serde_json::json!({
            "id": key_id,
            "legend": [l0.clone(), l1.clone(), l2.clone(), l3.clone()],
            "size": 1,
            "finger": finger
        }));
    }
    bot_row.push(serde_json::json!({"id": "rshift", "legend": ["Shift"], "size": 2.75, "finger": 9}));
    keys.push(serde_json::Value::Array(bot_row));

    // 最下段
    keys.push(serde_json::json!([
        {"id": "ctrl", "legend": ["Ctrl"], "size": 1.25, "finger": 0},
        {"id": "win", "legend": ["Win"], "size": 1.25, "finger": 0},
        {"id": "alt", "legend": ["Alt"], "size": 1.25, "finger": 0},
        {"id": "space", "legend": ["Space"], "size": 6.25, "finger": 4},
        {"id": "ralt", "legend": ["Alt"], "size": 1.25, "finger": 9},
        {"id": "rwin", "legend": ["Win"], "size": 1.25, "finger": 9},
        {"id": "menu", "legend": ["Menu"], "size": 1.25, "finger": 9},
        {"id": "rctrl", "legend": ["Ctrl"], "size": 1.25, "finger": 9}
    ]));

    // conversionセクションを構築
    let mut conversion = serde_json::Map::new();

    // シフトキー位置を特定
    let mut star_key = "d";    // ★のデフォルト（左中指）→ Layer 2
    let mut hoshi_key = "k";   // ☆のデフォルト（右中指）→ Layer 1
    let mut diamond_key = "/"; // ◆のデフォルト（右小指）→ Layer 3

    for row in 0..ROWS {
        let cols = cols_for_row(row);
        for col in 0..cols.min(COLS) {
            if layout.layers[0][row][col] == "★" {
                star_key = QWERTY_KEYS[row][col];
            }
            if layout.layers[0][row][col] == "☆" {
                hoshi_key = QWERTY_KEYS[row][col];
            }
            if layout.layers[0][row][col] == "◆" {
                diamond_key = QWERTY_KEYS[row][col];
            }
        }
    }

    // Layer 0 (no shift)
    for row in 0..ROWS {
        let cols = cols_for_row(row);
        for col in 0..cols {
            let kana = &layout.layers[0][row][col];
            if kana == "★" || kana == "☆" || kana == "◆" || kana == "　" || kana == "\0" {
                continue;
            }
            let key = if col < COLS { QWERTY_KEYS[row][col] } else { "'" };
            conversion.insert(kana.clone(), serde_json::json!({
                "keys": [key],
                "shift": [],
                "type": "sim",
                "ime": true
            }));
        }
    }

    // Layer 1 (☆ shift)
    for row in 0..ROWS {
        let cols = cols_for_row(row);
        for col in 0..cols {
            let kana = &layout.layers[1][row][col];
            if kana == "★" || kana == "☆" || kana == "◆" || kana == "　" || kana == "\0" || kana == "゛" || kana == "゜" {
                continue;
            }
            let key = if col < COLS { QWERTY_KEYS[row][col] } else { "'" };
            if !conversion.contains_key(kana) {
                conversion.insert(kana.clone(), serde_json::json!({
                    "keys": [key],
                    "shift": [hoshi_key],
                    "type": "sim",
                    "ime": true,
                    "renzsft": false
                }));
            }
        }
    }

    // Layer 2 (★ shift)
    for row in 0..ROWS {
        let cols = cols_for_row(row);
        for col in 0..cols {
            let kana = &layout.layers[2][row][col];
            if kana == "★" || kana == "☆" || kana == "◆" || kana == "　" || kana == "\0" || kana == "゛" || kana == "゜" {
                continue;
            }
            let key = if col < COLS { QWERTY_KEYS[row][col] } else { "'" };
            if !conversion.contains_key(kana) {
                conversion.insert(kana.clone(), serde_json::json!({
                    "keys": [key],
                    "shift": [star_key],
                    "type": "sim",
                    "ime": true,
                    "renzsft": false
                }));
            }
        }
    }

    // Layer 3 (◆ shift)
    for row in 0..ROWS {
        let cols = cols_for_row(row);
        for col in 0..cols {
            let kana = &layout.layers[3][row][col];
            if kana == "★" || kana == "☆" || kana == "◆" || kana == "　" || kana == "\0" || kana == "゛" || kana == "゜" {
                continue;
            }
            let key = if col < COLS { QWERTY_KEYS[row][col] } else { "'" };
            if !conversion.contains_key(kana) {
                conversion.insert(kana.clone(), serde_json::json!({
                    "keys": [key],
                    "shift": [diamond_key],
                    "type": "sim",
                    "ime": true,
                    "renzsft": false
                }));
            }
        }
    }

    let json = serde_json::json!({
        "name": "新月配列 v2.0 (Shingetsu Layout)",
        "remark": "★/☆/◆ 3シフトレイヤー切替方式のかな配列。",
        "keys": keys,
        "conversion": conversion
    });

    match std::fs::write(path, serde_json::to_string_pretty(&json).unwrap()) {
        Ok(_) => println!("  Analyzer JSON: {:?}", path),
        Err(e) => eprintln!("  Analyzer JSON保存エラー: {}", e),
    }
}

/// hazkey用TSV形式でエクスポート
pub fn export_tsv(layout: &Layout, path: &Path, colemak: bool) {
    let keys = if colemak { &COLEMAK_KEYS } else { &QWERTY_KEYS };
    let mut lines = Vec::new();

    let layout_name = if colemak { "Colemak" } else { "QWERTY" };
    lines.push(format!("# 新月配列 v2.0 (Shingetsu) {} ANSI用 hazkey ローマ字テーブル", layout_name));

    // シフトキー位置を特定
    let mut star_key = "d";    // ★
    let mut hoshi_key = "k";   // ☆
    let mut diamond_key = "/"; // ◆

    for row in 0..ROWS {
        for col in 0..COLS {
            if layout.layers[0][row][col] == "★" {
                star_key = keys[row][col];
            }
            if layout.layers[0][row][col] == "☆" {
                hoshi_key = keys[row][col];
            }
            if layout.layers[0][row][col] == "◆" {
                diamond_key = keys[row][col];
            }
        }
    }

    lines.push(format!("# ★={} (Layer 2), ☆={} (Layer 1), ◆={} (Layer 3)",
        star_key, hoshi_key, diamond_key));
    lines.push("".to_string());

    // シフトキー定義
    lines.push("# シフト".to_string());
    lines.push(format!("{}\t★", star_key));
    lines.push(format!("{}\t☆", hoshi_key));
    lines.push(format!("{}\t◆", diamond_key));
    lines.push("".to_string());

    // No Shift
    lines.push("# No Shift (ベース)".to_string());
    for row in 0..ROWS {
        let cols = cols_for_row(row);
        for col in 0..cols {
            let kana = &layout.layers[0][row][col];
            if kana == "★" || kana == "☆" || kana == "◆" || kana == "　" || kana == "\0" {
                continue;
            }
            let key = if col < COLS { keys[row][col] } else { "'" };
            lines.push(format!("{}\t{}", key, kana));
        }
    }
    lines.push("".to_string());

    // ☆シフト (Layer 1)
    lines.push(format!("# ☆シフト ({}前置)", hoshi_key));
    for row in 0..ROWS {
        let cols = cols_for_row(row);
        for col in 0..cols {
            let kana = &layout.layers[1][row][col];
            if kana == "★" || kana == "☆" || kana == "◆" || kana == "　" || kana == "\0" || kana == "゛" || kana == "゜" {
                continue;
            }
            let key = if col < COLS { keys[row][col] } else { "'" };
            lines.push(format!("☆{}\t{}", key, kana));
        }
    }
    lines.push("".to_string());

    // ★シフト (Layer 2)
    lines.push(format!("# ★シフト ({}前置)", star_key));
    for row in 0..ROWS {
        let cols = cols_for_row(row);
        for col in 0..cols {
            let kana = &layout.layers[2][row][col];
            if kana == "★" || kana == "☆" || kana == "◆" || kana == "　" || kana == "\0" || kana == "゛" || kana == "゜" {
                continue;
            }
            let key = if col < COLS { keys[row][col] } else { "'" };
            lines.push(format!("★{}\t{}", key, kana));
        }
    }
    lines.push("".to_string());

    // ◆シフト (Layer 3)
    lines.push(format!("# ◆シフト ({}前置)", diamond_key));
    for row in 0..ROWS {
        let cols = cols_for_row(row);
        for col in 0..cols {
            let kana = &layout.layers[3][row][col];
            if kana == "★" || kana == "☆" || kana == "◆" || kana == "　" || kana == "\0" || kana == "゛" || kana == "゜" {
                continue;
            }
            let key = if col < COLS { keys[row][col] } else { "'" };
            lines.push(format!("◆{}\t{}", key, kana));
        }
    }

    let content = lines.join("\n") + "\n";
    match std::fs::write(path, content) {
        Ok(_) => println!("  TSV ({}): {:?}", layout_name, path),
        Err(e) => eprintln!("  TSV保存エラー: {}", e),
    }
}

/// Karabiner Elements用JSON形式でエクスポート
pub fn export_karabiner(layout: &Layout, path: &Path) {
    let kana_map = kana_to_romaji();
    let mut manipulators = Vec::new();

    // 各キーのmanipulatorを生成 (row 0, 2: 10 cols, row 1: 11 cols)
    for row in 0..ROWS {
        let cols = cols_for_row(row);
        for col in 0..cols {
            // col 10 is the apostrophe key
            let key = if col < COLS { QWERTY_KEYS[row][col] } else { "quote" };
            let l0_str = &layout.layers[0][row][col];
            let l1_str = &layout.layers[1][row][col];
            let l2_str = &layout.layers[2][row][col];
            let l3_str = &layout.layers[3][row][col];
            let l0 = l0_str.chars().next().unwrap_or('　');
            let l1 = l1_str.chars().next().unwrap_or('　');
            let l2 = l2_str.chars().next().unwrap_or('　');
            let l3 = l3_str.chars().next().unwrap_or('　');

            // ★キーの処理 (Layer 2へのシフト)
            if l0_str == "★" {
                // ベース状態: シフトモードに入る (shift_state = 2 for ★)
                manipulators.push(serde_json::json!({
                    "conditions": [
                        {"input_sources": [{"language": "ja"}], "type": "input_source_if"},
                        {"input_sources": [{"input_mode_id": "Roman$"}], "type": "input_source_unless"}
                    ],
                    "from": {"key_code": key, "modifiers": {"optional": ["caps_lock"]}},
                    "to": [
                        {"set_variable": {"name": "last_char", "value": 0}},
                        {"set_variable": {"name": "shift_state", "value": 2}}
                    ],
                    "type": "basic"
                }));
                continue;
            }

            // ☆キーの処理 (Layer 1へのシフト)
            if l0_str == "☆" {
                // ベース状態: シフトモードに入る (shift_state = 1 for ☆)
                manipulators.push(serde_json::json!({
                    "conditions": [
                        {"input_sources": [{"language": "ja"}], "type": "input_source_if"},
                        {"input_sources": [{"input_mode_id": "Roman$"}], "type": "input_source_unless"}
                    ],
                    "from": {"key_code": key, "modifiers": {"optional": ["caps_lock"]}},
                    "to": [
                        {"set_variable": {"name": "last_char", "value": 0}},
                        {"set_variable": {"name": "shift_state", "value": 1}}
                    ],
                    "type": "basic"
                }));
                continue;
            }

            // ◆キーの処理 (Layer 3へのシフト)
            if l0_str == "◆" {
                // ベース状態: シフトモードに入る (shift_state = 3 for ◆)
                manipulators.push(serde_json::json!({
                    "conditions": [
                        {"input_sources": [{"language": "ja"}], "type": "input_source_if"},
                        {"input_sources": [{"input_mode_id": "Roman$"}], "type": "input_source_unless"}
                    ],
                    "from": {"key_code": key, "modifiers": {"optional": ["caps_lock"]}},
                    "to": [
                        {"set_variable": {"name": "last_char", "value": 0}},
                        {"set_variable": {"name": "shift_state", "value": 3}}
                    ],
                    "type": "basic"
                }));
                continue;
            }

            // 通常キーの処理

            // Layer 3 (◆シフト状態)
            if !matches!(l3, '　' | '\0' | '゛' | '゜') {
                if let Some(romaji) = kana_map.get(&l3) {
                    let mut to_keys = romaji_to_keycode(romaji);
                    to_keys.push(serde_json::json!({"set_variable": {"name": "last_char", "value": 0}}));
                    to_keys.push(serde_json::json!({"set_variable": {"name": "shift_state", "value": 0}}));

                    manipulators.push(serde_json::json!({
                        "conditions": [
                            {"type": "variable_if", "name": "shift_state", "value": 3},
                            {"input_sources": [{"language": "ja"}], "type": "input_source_if"},
                            {"input_sources": [{"input_mode_id": "Roman$"}], "type": "input_source_unless"}
                        ],
                        "from": {"key_code": key, "modifiers": {"optional": ["caps_lock"]}},
                        "to": to_keys,
                        "type": "basic"
                    }));
                }
            }

            // Layer 2 (★シフト状態)
            if !matches!(l2, '　' | '\0' | '゛' | '゜') {
                if let Some(romaji) = kana_map.get(&l2) {
                    let mut to_keys = romaji_to_keycode(romaji);
                    to_keys.push(serde_json::json!({"set_variable": {"name": "last_char", "value": 0}}));
                    to_keys.push(serde_json::json!({"set_variable": {"name": "shift_state", "value": 0}}));

                    manipulators.push(serde_json::json!({
                        "conditions": [
                            {"type": "variable_if", "name": "shift_state", "value": 2},
                            {"input_sources": [{"language": "ja"}], "type": "input_source_if"},
                            {"input_sources": [{"input_mode_id": "Roman$"}], "type": "input_source_unless"}
                        ],
                        "from": {"key_code": key, "modifiers": {"optional": ["caps_lock"]}},
                        "to": to_keys,
                        "type": "basic"
                    }));
                }
            }

            // Layer 1 (☆シフト状態)
            if !matches!(l1, '　' | '\0' | '゛' | '゜') {
                if let Some(romaji) = kana_map.get(&l1) {
                    let mut to_keys = romaji_to_keycode(romaji);
                    to_keys.push(serde_json::json!({"set_variable": {"name": "last_char", "value": 0}}));
                    to_keys.push(serde_json::json!({"set_variable": {"name": "shift_state", "value": 0}}));

                    manipulators.push(serde_json::json!({
                        "conditions": [
                            {"type": "variable_if", "name": "shift_state", "value": 1},
                            {"input_sources": [{"language": "ja"}], "type": "input_source_if"},
                            {"input_sources": [{"input_mode_id": "Roman$"}], "type": "input_source_unless"}
                        ],
                        "from": {"key_code": key, "modifiers": {"optional": ["caps_lock"]}},
                        "to": to_keys,
                        "type": "basic"
                    }));
                }
            }

            // Layer 0 (ベース状態)
            if !matches!(l0, '　' | '\0' | '゛' | '゜') {
                if let Some(romaji) = kana_map.get(&l0) {
                    let mut to_keys = romaji_to_keycode(romaji);
                    to_keys.push(serde_json::json!({"set_variable": {"name": "last_char", "value": 0}}));

                    manipulators.push(serde_json::json!({
                        "conditions": [
                            {"input_sources": [{"language": "ja"}], "type": "input_source_if"},
                            {"input_sources": [{"input_mode_id": "Roman$"}], "type": "input_source_unless"}
                        ],
                        "from": {"key_code": key, "modifiers": {"optional": ["caps_lock"]}},
                        "to": to_keys,
                        "type": "basic"
                    }));
                }
            }
        }
    }

    let json = serde_json::json!({
        "description": "新月配列 v2.0 (Shingetsu Layout) ☆/★/◆レイヤー切替方式",
        "manipulators": manipulators
    });

    match std::fs::write(path, serde_json::to_string_pretty(&json).unwrap()) {
        Ok(_) => println!("  Karabiner: {:?}", path),
        Err(e) => eprintln!("  Karabiner保存エラー: {}", e),
    }
}
