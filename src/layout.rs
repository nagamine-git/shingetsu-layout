//! 配列データ構造モジュール
//!
//! キーボード配列を表現するためのデータ構造を提供する。

use rand::seq::SliceRandom;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// 定数
// ============================================================================

/// キーボードの行数
pub const ROWS: usize = 3;

/// キーボードの列数（Row 0, 2用）
pub const COLS: usize = 10;

/// Row 1の列数（追加キー含む）
pub const COLS_ROW1: usize = 11;

/// レイヤー数（無シフト、☆シフト、★シフト、◆シフト）
pub const NUM_LAYERS: usize = 4;

/// 1レイヤーあたりのキー数 (10 + 11 + 10 = 31)
pub const KEYS_PER_LAYER: usize = COLS + COLS_ROW1 + COLS;

/// 行ごとの列数を取得
pub fn cols_for_row(row: usize) -> usize {
    if row == 1 { COLS_ROW1 } else { COLS }
}

/// キー配置ペナルティ（位置コスト）
/// Row 0: 10列, Row 1: 11列, Row 2: 10列
/// 0.0 は固定位置または空白位置
/// Layer 0: ★(row1,col2), ☆(row1,col7), ◆(row2,col9), 、(row2,col7), 。(row2,col8), ー(row1,col10)
/// Layer 1: ・(row1,col10)
/// Layer 2: blank(row1,col10)
/// Layer 3: ;(row1,col10), blank(col9 all rows)
pub const POSITION_COSTS_L0: [[f64; COLS_ROW1]; ROWS] = [
    // Row 0 (10 cols, last element unused)
    [3.7, 2.0, 2.0, 2.4, 3.5, 3.9, 2.4, 2.0, 2.0, 3.7, 0.0],
    // Row 1 (11 cols): ★=col2, ☆=col7, ー=col10
    [1.5, 1.0, 0.0, 1.0, 2.4, 2.4, 1.0, 0.0, 1.0, 1.5, 0.0],
    // Row 2 (10 cols): 、=col7, 。=col8, ◆=col9, last element unused
    [3.7, 2.8, 2.4, 2.0, 3.9, 3.0, 2.0, 0.0, 0.0, 0.0, 0.0],
];

pub const POSITION_COSTS_L1: [[f64; COLS_ROW1]; ROWS] = [
    // Row 0 (10 cols)
    [10.9, 5.8, 5.8, 7.1, 10.0, 11.2, 7.1, 17.4, 11.6, 21.7, 0.0],
    // Row 1 (11 cols): ・=col10
    [4.4, 2.9, 2.9, 2.9, 7.1, 7.1, 2.9, 5.8, 5.8, 8.7, 0.0],
    // Row 2 (10 cols)
    [10.9, 8.2, 7.1, 5.8, 11.2, 8.7, 5.8, 21.3, 16.4, 21.7, 0.0],
];

pub const POSITION_COSTS_L2: [[f64; COLS_ROW1]; ROWS] = [
    // Row 0 (10 cols)
    [21.7, 11.6, 17.4, 7.1, 10.0, 11.2, 7.1, 5.8, 5.8, 10.9, 0.0],
    // Row 1 (11 cols): blank=col10
    [8.7, 5.8, 5.8, 2.9, 7.1, 7.1, 2.9, 2.9, 2.9, 4.4, 0.0],
    // Row 2 (10 cols)
    [21.7, 16.4, 21.3, 5.8, 11.2, 8.7, 5.8, 7.1, 8.2, 10.9, 0.0],
];

pub const POSITION_COSTS_L3: [[f64; COLS_ROW1]; ROWS] = [
    // Row 0 (10 cols): blank=col9
    [80.8, 43.2, 43.2, 52.9, 74.8, 83.7, 52.9, 43.2, 43.2, 0.0, 0.0],
    // Row 1 (11 cols): blank=col9, ;=col10
    [32.4, 21.6, 21.6, 21.6, 52.9, 52.9, 21.6, 21.6, 21.6, 0.0, 0.0],
    // Row 2 (10 cols): blank=col9
    [80.8, 61.1, 52.9, 43.2, 83.7, 64.8, 43.2, 52.9, 61.1, 0.0, 0.0],
];

/// 全レイヤーの位置コストを取得
pub fn get_position_cost(layer: usize, row: usize, col: usize) -> f64 {
    match layer {
        0 => POSITION_COSTS_L0[row][col],
        1 => POSITION_COSTS_L1[row][col],
        2 => POSITION_COSTS_L2[row][col],
        3 => POSITION_COSTS_L3[row][col],
        _ => 0.0,
    }
}

/// ひらがな文字のデフォルト頻度順リスト（フォールバック用）
/// 112文字: 1gram(73) + 小書き(5:ぁぃぅぇぉ) + ゃゅょ終わり2gram(34)
/// 固定文字(8個): ★, ☆, ◆, 、, 。, ー, ・, ;
/// 配置可能位置: 124 - 8(固定) - 4(空白) = 112
/// Note: ゃ, ょ は 2gram でのみ使用（単独配置なし）
pub const HIRAGANA_FREQ_DEFAULT: &[&str] = &[
    // 1gram (73文字) - ー は固定文字なので含まない
    "い", "う", "ん", "し", "か", "の", "と", "た", "て", "く",
    "な", "に", "き", "は", "こ", "る", "が", "で", "っ", "す",
    "ま", "じ", "り", "も", "つ", "お", "ら", "を", "さ", "あ",
    "れ", "だ", "ち", "せ", "け", "よ", "ど", "そ", "え", "わ",
    "み", "め", "ひ", "ば", "や", "ろ", "ほ", "ふ", "ぶ", "ね",
    "ご", "ぎ", "げ", "む", "ず", "び", "ざ", "ぐ", "ぜ", "へ",
    "べ", "ゆ", "ぼ", "ぷ", "ぞ", "ぱ", "ぽ", "づ", "ぴ", "ぬ",
    "ぺ", "ヴ", "ぢ",
    // 小書き (5文字) - ゃ, ょ は2gramでのみ使用
    "ぁ", "ぃ", "ぅ", "ぇ", "ぉ",
    // ゃゅょ終わり2gram (34文字)
    "しょ", "じょ", "しゅ", "きょ", "しゃ", "ちょ", "じゅ", "りょ",
    "きゅ", "ちゅ", "ぎょ", "にゅ", "ひょ", "じゃ", "ちゃ", "りゅ",
    "きゃ", "びょ", "りゃ", "ぎゃ", "ぴょ", "ぴゅ", "びゅ", "みょ",
    "ひゃ", "みゅ", "にょ", "みゃ", "にゃ", "ひゅ", "びゃ", "ぴゃ",
    "ぎゅ", "ぢゃ",
];

// ============================================================================
// キー位置
// ============================================================================

/// キーの位置を表す構造体
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyPos {
    /// レイヤー（0: 無シフト, 1: ☆シフト, 2: ★シフト, 3: ◆シフト）
    pub layer: usize,
    /// 行（0: 上段, 1: 中段, 2: 下段）
    pub row: usize,
    /// 列（Row0,2: 0-9, Row1: 0-10）
    pub col: usize,
}

impl KeyPos {
    /// 新しいキー位置を作成
    pub fn new(layer: usize, row: usize, col: usize) -> Self {
        Self { layer, row, col }
    }

    /// ホームポジションかどうか（中段）
    pub fn is_home(&self) -> bool {
        self.row == 1
    }

    /// 左手かどうか（列0-4）
    pub fn is_left_hand(&self) -> bool {
        self.col < 5
    }

    /// 指のインデックス（0: 小指, 1: 薬指, 2: 中指, 3: 人差し指）
    pub fn finger(&self) -> usize {
        match self.col {
            0 | 9 => 0, // 小指
            1 | 8 => 1, // 薬指
            2 | 7 => 2, // 中指
            3 | 4 | 5 | 6 => 3, // 人差し指
            _ => 0,
        }
    }

    /// キーの打鍵コスト（距離ベース）
    pub fn weight(&self) -> f64 {
        // 基本重み: ホームポジション = 1.0
        let row_weight = match self.row {
            1 => 1.0,  // ホーム
            0 => 1.3,  // 上段
            2 => 1.2,  // 下段
            _ => 2.0,
        };

        // 列の重み（中央が低い）
        let col_weight = match self.col {
            3 | 4 | 5 | 6 => 1.0,  // 人差し指
            2 | 7 => 1.1,          // 中指
            1 | 8 => 1.2,          // 薬指
            0 | 9 => 1.4,          // 小指
            10 => 1.5,             // 追加キー（row1のみ）
            _ => 1.5,
        };

        // シフトレイヤーのペナルティ
        let layer_weight = match self.layer {
            0 => 1.0,       // 無シフト
            1 => 2.0,       // 中指シフト（☆）
            2 => 2.2,       // 中指シフト（★）
            3 => 3.0,       // 薬指シフト（◆）
            _ => 3.0,
        };

        row_weight * col_weight * layer_weight
    }
}

// ============================================================================
// 配列
// ============================================================================

/// キーボード配列を表す構造体
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Layout {
    /// 4層の配列データ [layer][row][col] - String型（1文字 or 2文字の拗音対応）
    /// Row0: 10列, Row1: 11列, Row2: 10列
    pub layers: Vec<Vec<Vec<String>>>,

    /// 評価フィットネス値
    #[serde(default)]
    pub fitness: f64,

    /// 詳細スコア
    #[serde(default)]
    pub scores: EvaluationScores,
}

/// 評価スコアの詳細
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EvaluationScores {
    /// 段飛ばしの少なさ（高いほど良い）
    pub row_skip: f64,
    /// ホームポジション率
    pub home_position: f64,
    /// 総打鍵コストの低さ
    pub total_keystrokes: f64,
    /// 同指連続の少なさ
    pub same_finger: f64,
    /// 単打鍵率（シフト無し）
    pub single_key: f64,
    /// Colemak類似度
    pub colemak_similarity: f64,
    /// 位置別コスト（ベースコスト×シフト係数）
    pub position_cost: f64,
    /// 月配列類似度
    pub tsuki_similarity: f64,
    /// 覚えやすさ
    pub memorability: f64,
    /// 左右交互打鍵率
    pub alternating: f64,
    /// ロール率
    pub roll: f64,
    /// リダイレクト少なさ
    pub redirect_low: f64,
    /// インロール率
    pub inroll: f64,
    /// アルペジオ率
    pub arpeggio: f64,
    /// シフトバランス（☆★均等）
    pub shift_balance: f64,
}

impl Default for Layout {
    fn default() -> Self {
        let layers = (0..NUM_LAYERS)
            .map(|_| {
                (0..ROWS)
                    .map(|row| {
                        let cols = cols_for_row(row);
                        (0..cols)
                            .map(|_| "　".to_string())
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        Self {
            layers,
            fitness: 0.0,
            scores: EvaluationScores::default(),
        }
    }
}

impl Layout {
    /// 改善版カスタムレイアウト（初期配置として使用）
    /// シフトキー: ★(row1,col2) → Layer2, ☆(row1,col7) → Layer1, ◆(row2,col9) → Layer3
    /// 固定文字(8個): ★, ☆, ◆, 、, 。, ー, ・, ;
    pub fn improved_custom() -> Self {
        let mut layers: Vec<Vec<Vec<String>>> = (0..NUM_LAYERS)
            .map(|_| {
                (0..ROWS)
                    .map(|row| {
                        let cols = cols_for_row(row);
                        vec!["　".to_string(); cols]
                    })
                    .collect()
            })
            .collect();

        // Layer 0 (No Shift)
        // Row 0: 10 cols
        layers[0][0] = vec!["あ", "と", "に", "る", "を", "ち", "こ", "く", "て", "さ"]
            .iter().map(|s| s.to_string()).collect();
        // Row 1: 11 cols (★=col2, ☆=col7, ー=col10)
        layers[0][1] = vec!["か", "う", "★", "し", "た", "き", "ん", "☆", "い", "の", "ー"]
            .iter().map(|s| s.to_string()).collect();
        // Row 2: 10 cols (、=col7, 。=col8, ◆=col9)
        layers[0][2] = vec!["れ", "で", "が", "な", "だ", "ら", "は", "、", "。", "◆"]
            .iter().map(|s| s.to_string()).collect();

        // Layer 1 (☆シフト) - row1,col7で発動
        // Row 0: 10 cols
        layers[1][0] = vec!["べ", "ど", "わ", "しゅ", "ぐ", "ぞ", "ぎ", "ぽ", "きゅ", "ぺ"]
            .iter().map(|s| s.to_string()).collect();
        // Row 1: 11 cols (・=col10)
        layers[1][1] = vec!["せ", "じ", "す", "つ", "ぶ", "ふ", "っ", "ほ", "ろ", "ざ", "・"]
            .iter().map(|s| s.to_string()).collect();
        // Row 2: 10 cols
        layers[1][2] = vec!["じゅ", "ず", "む", "ひ", "ぷ", "び", "み", "にゅ", "ちゅ", "ぬ"]
            .iter().map(|s| s.to_string()).collect();

        // Layer 2 (★シフト) - row1,col2で発動
        // Row 0: 10 cols
        layers[2][0] = vec!["りゅ", "ぱ", "ぃ", "げ", "ぜ", "りょ", "ご", "や", "め", "ゆ"]
            .iter().map(|s| s.to_string()).collect();
        // Row 1: 11 cols (blank=col10)
        layers[2][1] = vec!["ちょ", "そ", "よ", "も", "ね", "しょ", "ま", "り", "お", "け", "　"]
            .iter().map(|s| s.to_string()).collect();
        // Row 2: 10 cols
        layers[2][2] = vec!["きゃ", "ぎょ", "ひょ", "ば", "ぼ", "しゃ", "え", "じょ", "きょ", "へ"]
            .iter().map(|s| s.to_string()).collect();

        // Layer 3 (◆シフト) - row2,col9で発動
        // Row 0: 10 cols (blank=col9)
        layers[3][0] = vec!["ひゅ", "りゃ", "ぎゃ", "みゅ", "にゃ", "ぢゃ", "みょ", "ヴ", "ぴょ", "　"]
            .iter().map(|s| s.to_string()).collect();
        // Row 1: 11 cols (blank=col9, ;=col10)
        layers[3][1] = vec!["びょ", "じゃ", "ぇ", "ぴ", "ぢ", "びゅ", "づ", "ぁ", "ちゃ", "　", ";"]
            .iter().map(|s| s.to_string()).collect();
        // Row 2: 10 cols (blank=col9)
        layers[3][2] = vec!["びゃ", "ぅ", "ぎゅ", "ぴゅ", "ぴゃ", "みゃ", "ぉ", "ひゃ", "にょ", "　"]
            .iter().map(|s| s.to_string()).collect();

        Self {
            layers,
            fitness: 0.0,
            scores: EvaluationScores::default(),
        }
    }
    
    /// ランダムな配列を生成（デフォルト頻度リスト使用）
    pub fn random(rng: &mut ChaCha8Rng) -> Self {
        Self::random_with_chars(rng, HIRAGANA_FREQ_DEFAULT)
    }
    
    /// 指定した文字リストからランダムな配列を生成
    /// コーパスの1gramから取得した頻度順リストを使用可能（1文字 or 2文字の拗音対応）
    pub fn random_with_chars(rng: &mut ChaCha8Rng, hiragana_chars: &[&str]) -> Self {
        let mut chars: Vec<String> = hiragana_chars.iter().map(|s| s.to_string()).collect();

        // 固定位置 (8):
        //   Layer 0: ★,☆,◆ (3) + 、,。,ー (3) = 6
        //   Layer 1: ・ (1) = 1
        //   Layer 3: ; (1) = 1
        // シフト制限空白 (4):
        //   Layer 2: row1,col10 = 1
        //   Layer 3: col9 all rows = 3
        // 配置可能: 124 - 8 - 4 = 112ポジション
        const FIXED_COUNT: usize = 8;
        const SHIFT_BLANK_COUNT: usize = 4;
        let total_positions = KEYS_PER_LAYER * NUM_LAYERS - FIXED_COUNT - SHIFT_BLANK_COUNT;

        // 112個分の文字を用意（足りなければ空白で埋める）
        while chars.len() < total_positions {
            chars.push("　".to_string());
        }
        // 多すぎる場合は切り詰め
        chars.truncate(total_positions);

        chars.shuffle(rng);

        let mut layers: Vec<Vec<Vec<String>>> = (0..NUM_LAYERS)
            .map(|_| {
                (0..ROWS)
                    .map(|row| {
                        let cols = cols_for_row(row);
                        vec!["　".to_string(); cols]
                    })
                    .collect()
            })
            .collect();

        // 固定文字の配置
        // Layer 0: シフトキーと句読点、長音符
        layers[0][1][2] = "★".to_string();   // Layer 2
        layers[0][1][7] = "☆".to_string();   // Layer 1
        layers[0][2][9] = "◆".to_string();   // Layer 3
        layers[0][2][7] = "、".to_string();
        layers[0][2][8] = "。".to_string();
        layers[0][1][10] = "ー".to_string();
        // Layer 1: 中黒
        layers[1][1][10] = "・".to_string();
        // Layer 3: セミコロン
        layers[3][1][10] = ";".to_string();

        // シャッフルした文字を配置（固定位置と空白位置を除く112ポジション）
        let mut char_idx = 0;
        for layer in 0..NUM_LAYERS {
            for row in 0..ROWS {
                let cols = cols_for_row(row);
                for col in 0..cols {
                    // 固定位置と空白位置をスキップ
                    if !Self::is_fixed_position(layer, row, col) && !Self::is_blank_position(layer, row, col) {
                        if char_idx < chars.len() {
                            layers[layer][row][col] = chars[char_idx].clone();
                            char_idx += 1;
                        }
                    }
                }
            }
        }

        // デバッグ確認
        debug_assert_eq!(char_idx, chars.len(),
            "配置した文字数({})と用意した文字数({})が不一致", char_idx, chars.len());

        Self {
            layers,
            fitness: 0.0,
            scores: EvaluationScores::default(),
        }
    }

    /// シフト制限による空白位置かどうかを判定
    /// Layer 2, row1, col10: blank (★と同時押し不可)
    /// Layer 3, col9: blank (◆と同じ列で押せない)
    pub fn is_blank_position(layer: usize, row: usize, col: usize) -> bool {
        match layer {
            // Layer 2 (★シフト): row1,col10のみ空白
            2 => {
                row == 1 && col == 10
            }
            // Layer 3 (◆シフト): col9が全row空白
            3 => {
                col == 9
            }
            _ => false,
        }
    }

    /// 固定位置かどうかを判定
    /// 固定文字(8個): ★, ☆, ◆ (シフトキー) + 、, 。, ー (Layer0) + ・ (Layer1) + ; (Layer3)
    pub fn is_fixed_position(layer: usize, row: usize, col: usize) -> bool {
        // Layer 0：シフトキー位置（★=row1,col2, ☆=row1,col7, ◆=row2,col9）
        if layer == 0 && row == 1 && (col == 2 || col == 7) {
            return true;
        }
        if layer == 0 && row == 2 && col == 9 {
            return true;  // ◆
        }
        // Layer 0：句読点（、=row2,col7, 。=row2,col8）
        if layer == 0 && row == 2 && (col == 7 || col == 8) {
            return true;
        }
        // Layer 0：長音符（ー=row1,col10）
        if layer == 0 && row == 1 && col == 10 {
            return true;
        }
        // Layer 1：中黒（・=row1,col10）
        if layer == 1 && row == 1 && col == 10 {
            return true;
        }
        // Layer 3：セミコロン（;=row1,col10）
        if layer == 3 && row == 1 && col == 10 {
            return true;
        }
        false
    }

    /// 文字の位置を検索
    pub fn find_char(&self, c: &str) -> Option<KeyPos> {
        for layer in 0..NUM_LAYERS {
            for row in 0..ROWS {
                let cols = cols_for_row(row);
                for col in 0..cols {
                    if self.layers[layer][row][col] == c {
                        return Some(KeyPos::new(layer, row, col));
                    }
                }
            }
        }
        None
    }

    /// 文字→位置のマップを構築
    /// 2gram文字列の場合は最初の文字のみをキーとして使用
    pub fn build_char_map(&self) -> HashMap<char, KeyPos> {
        let mut map = HashMap::new();
        for layer in 0..NUM_LAYERS {
            for row in 0..ROWS {
                let cols = cols_for_row(row);
                for col in 0..cols {
                    let s = &self.layers[layer][row][col];
                    if let Some(c) = s.chars().next() {
                        if c != '　' && c != '\0' {
                            map.entry(c).or_insert(KeyPos::new(layer, row, col));
                        }
                    }
                }
            }
        }
        map
    }

    /// 配列を整形して文字列で返す
    pub fn format(&self) -> String {
        let mut result = String::new();

        for layer in 0..NUM_LAYERS {
            let label = match layer {
                0 => "Layer 0 (無シフト)",
                1 => "Layer 1 (☆シフト)",
                2 => "Layer 2 (★シフト)",
                3 => "Layer 3 (◆シフト)",
                _ => "Unknown",
            };
            result.push_str(&format!("{}:\n", label));

            for row in 0..ROWS {
                result.push_str("  ");
                let cols = cols_for_row(row);
                for col in 0..cols {
                    let s = &self.layers[layer][row][col];
                    result.push_str(s);
                    result.push(' ');
                }
                result.push('\n');
            }
            result.push('\n');
        }

        result
    }

    /// 配列の検証（重複・不足チェック）
    /// Returns: (duplicates, missing, extra) - 問題があれば該当文字のリスト
    pub fn validate(&self, expected_chars: &[&str]) -> ValidationResult {
        use std::collections::{HashMap as StdHashMap, HashSet};

        let mut char_counts: StdHashMap<String, Vec<(usize, usize, usize)>> = StdHashMap::new();
        let mut found_chars: HashSet<String> = HashSet::new();

        // 全ポジションをスキャン
        for layer in 0..NUM_LAYERS {
            for row in 0..ROWS {
                let cols = cols_for_row(row);
                for col in 0..cols {
                    // 固定位置と空白位置はスキップ
                    if Self::is_fixed_position(layer, row, col) {
                        continue;
                    }
                    if Self::is_blank_position(layer, row, col) {
                        continue;
                    }

                    let s = &self.layers[layer][row][col];
                    if s != "　" && !s.is_empty() {
                        char_counts.entry(s.clone()).or_default().push((layer, row, col));
                        found_chars.insert(s.clone());
                    }
                }
            }
        }

        // 重複チェック
        let duplicates: Vec<(String, Vec<(usize, usize, usize)>)> = char_counts
            .iter()
            .filter(|(_, positions)| positions.len() > 1)
            .map(|(c, positions)| (c.clone(), positions.clone()))
            .collect();

        // 期待文字セット
        let expected: HashSet<String> = expected_chars.iter().map(|s| s.to_string()).collect();

        // 不足チェック（期待されているが見つからない）
        let missing: Vec<String> = expected
            .difference(&found_chars)
            .cloned()
            .collect();

        // 余分チェック（見つかったが期待されていない）
        let extra: Vec<String> = found_chars
            .difference(&expected)
            .cloned()
            .collect();

        ValidationResult {
            duplicates,
            missing,
            extra,
            total_found: found_chars.len(),
            total_expected: expected.len(),
        }
    }
}

/// 配列検証結果
#[derive(Debug)]
pub struct ValidationResult {
    /// 重複している文字と位置
    pub duplicates: Vec<(String, Vec<(usize, usize, usize)>)>,
    /// 不足している文字
    pub missing: Vec<String>,
    /// 余分な文字（期待リストにない）
    pub extra: Vec<String>,
    /// 見つかった文字数
    pub total_found: usize,
    /// 期待される文字数
    pub total_expected: usize,
}

impl ValidationResult {
    /// 重複がないかどうか（メインの検証基準）
    pub fn is_valid(&self) -> bool {
        self.duplicates.is_empty()
    }

    pub fn print_report(&self) {
        println!("\n=== 配列検証結果 ===");
        println!("配置文字数: {} (配置可能: 112)", self.total_found);

        if self.duplicates.is_empty() {
            println!("✓ 重複なし");
        } else {
            println!("✗ 重複あり ({} 件):", self.duplicates.len());
            for (c, positions) in &self.duplicates {
                let pos_str: Vec<String> = positions
                    .iter()
                    .map(|(l, r, c)| format!("L{}[{}][{}]", l, r, c))
                    .collect();
                println!("  「{}」: {}", c, pos_str.join(", "));
            }
        }

        // 不足は参考情報（119ポジションに135文字は入らないので）
        if !self.missing.is_empty() {
            println!("※ 未配置文字 ({} 件): 配置枠より文字数が多いため正常", self.missing.len());
        }

        if !self.extra.is_empty() {
            println!("※ 期待リスト外の文字 ({} 件):", self.extra.len());
            for c in &self.extra {
                print!("「{}」", c);
            }
            println!();
        }

        if self.is_valid() {
            println!("\n✓ 検証成功: 重複なし、配列は正常です");
        } else {
            println!("\n✗ 検証失敗: 重複があります");
        }
    }
}

// ============================================================================
// 月配列参照データ
// ============================================================================

/// 月配列の位置情報
pub struct TsukiLayout {
    pub char_positions: HashMap<char, KeyPos>,
}

impl TsukiLayout {
    /// 月配列2-263の配置を生成
    pub fn new() -> Self {
        let mut positions = HashMap::new();
        
        // Layer 0（表面）
        let layer0 = [
            ['そ', 'こ', 'し', 'て', 'ょ', 'つ', 'ん', 'い', 'の', 'り'],
            ['は', 'か', '☆', 'と', 'た', 'く', 'う', '★', '゛', 'き'],
            ['す', 'け', 'に', 'な', 'さ', 'っ', 'る', '、', '。', '゜'],
        ];
        
        // Layer 1（裏面）
        let layer1 = [
            ['ぁ', 'ひ', 'ほ', 'ふ', 'め', 'ぬ', 'え', 'み', 'や', 'ぇ'],
            ['ぃ', 'を', 'ら', 'あ', 'よ', 'ま', 'お', 'も', 'わ', 'ゆ'],
            ['ぅ', 'へ', 'せ', 'ゅ', 'ゃ', 'む', 'ろ', 'ね', 'ー', 'ぉ'],
        ];
        
        for (row, chars) in layer0.iter().enumerate() {
            for (col, &c) in chars.iter().enumerate() {
                if c != '☆' && c != '★' && c != '゛' && c != '゜' {
                    positions.insert(c, KeyPos::new(0, row, col));
                }
            }
        }
        
        for (row, chars) in layer1.iter().enumerate() {
            for (col, &c) in chars.iter().enumerate() {
                positions.insert(c, KeyPos::new(1, row, col));
            }
        }
        
        Self { char_positions: positions }
    }
}

impl Default for TsukiLayout {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// ローマ字音素マッピング
// ============================================================================

/// かな文字からローマ字音素への分解
/// 戻り値: (子音, 母音)
pub fn romaji_phonemes(c: char) -> (Option<&'static str>, Option<&'static str>) {
    match c {
        'あ' => (None, Some("a")),
        'い' => (None, Some("i")),
        'う' => (None, Some("u")),
        'え' => (None, Some("e")),
        'お' => (None, Some("o")),
        'か' => (Some("k"), Some("a")),
        'き' => (Some("k"), Some("i")),
        'く' => (Some("k"), Some("u")),
        'け' => (Some("k"), Some("e")),
        'こ' => (Some("k"), Some("o")),
        'さ' => (Some("s"), Some("a")),
        'し' => (Some("s"), Some("i")),
        'す' => (Some("s"), Some("u")),
        'せ' => (Some("s"), Some("e")),
        'そ' => (Some("s"), Some("o")),
        'た' => (Some("t"), Some("a")),
        'ち' => (Some("t"), Some("i")),
        'つ' => (Some("t"), Some("u")),
        'て' => (Some("t"), Some("e")),
        'と' => (Some("t"), Some("o")),
        'な' => (Some("n"), Some("a")),
        'に' => (Some("n"), Some("i")),
        'ぬ' => (Some("n"), Some("u")),
        'ね' => (Some("n"), Some("e")),
        'の' => (Some("n"), Some("o")),
        'は' => (Some("h"), Some("a")),
        'ひ' => (Some("h"), Some("i")),
        'ふ' => (Some("h"), Some("u")),
        'へ' => (Some("h"), Some("e")),
        'ほ' => (Some("h"), Some("o")),
        'ま' => (Some("m"), Some("a")),
        'み' => (Some("m"), Some("i")),
        'む' => (Some("m"), Some("u")),
        'め' => (Some("m"), Some("e")),
        'も' => (Some("m"), Some("o")),
        'や' => (Some("y"), Some("a")),
        'ゆ' => (Some("y"), Some("u")),
        'よ' => (Some("y"), Some("o")),
        'ら' => (Some("r"), Some("a")),
        'り' => (Some("r"), Some("i")),
        'る' => (Some("r"), Some("u")),
        'れ' => (Some("r"), Some("e")),
        'ろ' => (Some("r"), Some("o")),
        'わ' => (Some("w"), Some("a")),
        'を' => (Some("w"), Some("o")),
        'ん' => (Some("n"), None),  // 「ん」は子音"n"のみ
        'が' => (Some("g"), Some("a")),
        'ぎ' => (Some("g"), Some("i")),
        'ぐ' => (Some("g"), Some("u")),
        'げ' => (Some("g"), Some("e")),
        'ご' => (Some("g"), Some("o")),
        'ざ' => (Some("z"), Some("a")),
        'じ' => (Some("z"), Some("i")),
        'ず' => (Some("z"), Some("u")),
        'ぜ' => (Some("z"), Some("e")),
        'ぞ' => (Some("z"), Some("o")),
        'だ' => (Some("d"), Some("a")),
        'ぢ' => (Some("d"), Some("i")),
        'づ' => (Some("d"), Some("u")),
        'で' => (Some("d"), Some("e")),
        'ど' => (Some("d"), Some("o")),
        'ば' => (Some("b"), Some("a")),
        'び' => (Some("b"), Some("i")),
        'ぶ' => (Some("b"), Some("u")),
        'べ' => (Some("b"), Some("e")),
        'ぼ' => (Some("b"), Some("o")),
        'ぱ' => (Some("p"), Some("a")),
        'ぴ' => (Some("p"), Some("i")),
        'ぷ' => (Some("p"), Some("u")),
        'ぺ' => (Some("p"), Some("e")),
        'ぽ' => (Some("p"), Some("o")),
        _ => (None, None),
    }
}

/// Colemakのキー位置マッピング
pub const COLEMAK_POSITIONS: &[(&str, usize, usize)] = &[
    // 母音位置
    ("a", 1, 0), ("e", 1, 7), ("i", 1, 8), ("o", 1, 9), ("u", 0, 7),
    // 子音位置
    ("k", 2, 6), ("s", 1, 2), ("t", 1, 3), ("n", 1, 6), ("h", 1, 5),
    ("m", 2, 7), ("y", 0, 8), ("r", 1, 1), ("w", 0, 1), ("g", 0, 4),
    ("z", 2, 0), ("d", 1, 4), ("b", 2, 4), ("p", 0, 3), ("f", 0, 2),
    ("j", 0, 5), ("l", 0, 6), ("v", 2, 3), ("q", 0, 0), ("x", 2, 1),
    ("c", 2, 2),
];
