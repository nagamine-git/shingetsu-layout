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

/// キーボードの列数
pub const COLS: usize = 10;

/// レイヤー数（無シフト、☆シフト、★シフト）
pub const NUM_LAYERS: usize = 3;

/// 1レイヤーあたりのキー数
pub const KEYS_PER_LAYER: usize = ROWS * COLS;

/// ひらがな文字のデフォルト頻度順リスト（フォールバック用）
/// 実際の頻度は1gramファイルから読み込まれる
pub const HIRAGANA_FREQ_DEFAULT: &[char] = &[
    // 高頻度（無シフト層候補）
    'い', 'う', 'ん', 'し', 'か', 'の', 'と', 'た', 'て', 'く',
    'な', 'に', 'き', 'は', 'こ', 'る', 'が', 'で', 'っ', 'ょ',
    'す', 'ま', 'じ', 'り', 'も', 'つ', 'お', 'ら', 'を', 'さ',
    // 中頻度（☆シフト層候補）
    'あ', 'れ', 'だ', 'ち', 'せ', 'け', 'ー', 'よ', 'ど', 'ゅ',
    'そ', 'え', 'わ', 'み', 'め', 'ひ', 'ば', 'や', 'ろ', 'ほ',
    'ふ', 'ゃ', 'ぶ', 'ね', 'ご', 'ぎ', 'げ', 'む', 'ず', 'び',
    // 低頻度（★シフト層候補）
    'ざ', 'ぐ', 'ぜ', 'へ', 'べ', 'ゆ', 'ぼ', 'ぷ', 'ぞ', 'ぱ',
    'ぃ', 'ぽ', 'ぇ', 'づ', 'ぴ', 'ぁ', 'ぬ', 'ぺ', 'ぉ', 'ぢ', 'ぅ', 'ゔ',
];

// ============================================================================
// キー位置
// ============================================================================

/// キーの位置を表す構造体
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyPos {
    /// レイヤー（0: 無シフト, 1: ☆シフト, 2: ★シフト）
    pub layer: usize,
    /// 行（0: 上段, 1: 中段, 2: 下段）
    pub row: usize,
    /// 列（0-9）
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
            _ => 1.5,
        };

        // シフトレイヤーのペナルティ
        let layer_weight = match self.layer {
            0 => 1.0,
            1 | 2 => 2.0,  // シフト必要
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
    /// 3層の配列データ [layer][row][col]
    pub layers: [[[char; COLS]; ROWS]; NUM_LAYERS],
    
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
        Self {
            layers: [[['　'; COLS]; ROWS]; NUM_LAYERS],
            fitness: 0.0,
            scores: EvaluationScores::default(),
        }
    }
}

impl Layout {
    /// 改善版カスタムレイアウト（初期配置として使用）
    pub fn improved_custom() -> Self {
        let mut layers = [[['　'; COLS]; ROWS]; NUM_LAYERS];
        
        // Layer 0 (No Shift) - 高頻度文字最適化
        layers[0] = [
            ['れ', 'た', 'な', 'に', 'お', 'ち', 'は', 'と', 'こ', 'を'],
            ['か', 'し', '☆', 'う', 'が', 'く', 'ん', '★', 'い', 'の'],
            ['あ', 'で', 'き', 'て', 'だ', 'っ', 'る', '、', '。', 'さ'],
        ];
        
        // Layer 1 (☆シフト) - ;・を固定位置に
        layers[1] = [
            ['ぷ', 'そ', 'み', 'ゃ', 'へ', 'ぽ', 'ふ', 'ょ', 'ぁ', 'わ'],
            ['け', 'つ', 'ら', 'す', 'ね', 'ぶ', 'め', 'ぐ', 'び', 'づ'],
            ['ぼ', 'ぎ', 'ほ', 'え', 'ぃ', 'ざ', 'ご', '；', '・', 'ぢ'],
        ];
        
        // Layer 2 (★シフト)
        layers[2] = [
            ['ぉ', 'ぴ', 'ぅ', 'ば', 'ぜ', 'ぱ', 'ひ', 'よ', 'ゅ', 'べ'],
            ['ぇ', 'ず', 'げ', 'ま', 'ろ', 'や', 'じ', 'も', 'り', 'せ'],
            ['ぺ', 'ぬ', 'ど', 'ぞ', 'む', 'ぶ', 'ゆ', 'ー', 'ゃ', '　'],
        ];
        
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
    /// コーパスの1gramから取得した頻度順リストを使用可能
    pub fn random_with_chars(rng: &mut ChaCha8Rng, hiragana_chars: &[char]) -> Self {
        let mut chars: Vec<char> = hiragana_chars.to_vec();
        
        // 90ポジション中、固定位置：
        // Layer 0: ★☆（2個）+、。（2個）
        // Layer 2: ;・（2個）
        // 実際に配置可能: 90 - 6 = 84ポジション
        let total_positions = KEYS_PER_LAYER * NUM_LAYERS - 6;
        while chars.len() < total_positions {
            chars.push('　');
        }
        // 多すぎる場合は切り詰め
        chars.truncate(total_positions);
        
        chars.shuffle(rng);
        
        let mut layers = [[['　'; COLS]; ROWS]; NUM_LAYERS];
        
        // 固定文字の配置
        // Layer 0: シフトキーと句読点
        layers[0][1][2] = '☆';  // sキー: ☆シフト
        layers[0][1][7] = '★';  // eキー: ★シフト
        layers[0][2][7] = '、';
        layers[0][2][8] = '。';
        
        // Layer 1（☆シフト）: 記号
        layers[1][2][7] = '；';  // セミコロン
        layers[1][2][8] = '・';  // 中黒
        
        // 残りの文字をシャッフルして配置
        let mut char_idx = 0;
        for layer in 0..NUM_LAYERS {
            for row in 0..ROWS {
                for col in 0..COLS {
                    if !Self::is_fixed_position(layer, row, col) {
                        if char_idx < chars.len() {
                            layers[layer][row][col] = chars[char_idx];
                            char_idx += 1;
                        }
                    }
                }
            }
        }
        
        Self {
            layers,
            fitness: 0.0,
            scores: EvaluationScores::default(),
        }
    }

    /// 固定位置かどうかを判定
    pub fn is_fixed_position(layer: usize, row: usize, col: usize) -> bool {
        // Layer 0：シフトキー位置（★s, ☆e）
        if layer == 0 && row == 1 && (col == 2 || col == 7) {
            return true;
        }
        // Layer 0：句読点（、。）
        if layer == 0 && row == 2 && (col == 7 || col == 8) {
            return true;
        }
        // Layer 1（☆シフト）：記号（;・）
        if layer == 1 && row == 2 && (col == 7 || col == 8) {
            return true;
        }
        false
    }

    /// 文字の位置を検索
    pub fn find_char(&self, c: char) -> Option<KeyPos> {
        for layer in 0..NUM_LAYERS {
            for row in 0..ROWS {
                for col in 0..COLS {
                    if self.layers[layer][row][col] == c {
                        return Some(KeyPos::new(layer, row, col));
                    }
                }
            }
        }
        None
    }

    /// 文字→位置のマップを構築
    pub fn build_char_map(&self) -> HashMap<char, KeyPos> {
        let mut map = HashMap::new();
        for layer in 0..NUM_LAYERS {
            for row in 0..ROWS {
                for col in 0..COLS {
                    let c = self.layers[layer][row][col];
                    if c != '　' && c != '\0' {
                        map.entry(c).or_insert(KeyPos::new(layer, row, col));
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
                _ => "Unknown",
            };
            result.push_str(&format!("{}:\n", label));
            
            for row in 0..ROWS {
                result.push_str("  ");
                for col in 0..COLS {
                    let c = self.layers[layer][row][col];
                    result.push(c);
                    result.push(' ');
                }
                result.push('\n');
            }
            result.push('\n');
        }
        
        result
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
