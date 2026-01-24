//! 評価モジュール
//! 
//! 配列の評価スコアを計算する。

use std::collections::{HashMap, HashSet};

use crate::corpus::CorpusStats;
use crate::layout::{
    romaji_phonemes, EvaluationScores, KeyPos, Layout, TsukiLayout,
    COLEMAK_POSITIONS, COLS, NUM_LAYERS, ROWS,
};

// ============================================================================
// 評価重み
// ============================================================================

/// 評価メトリクスの重み設定
#[derive(Debug, Clone)]
pub struct EvaluationWeights {
    // Core Metrics（乗算・指数）- 基本5指標
    pub same_finger: f64,          // 同指連続率の低さ
    pub row_skip: f64,             // 段越えの少なさ
    pub home_position: f64,        // ホームポジション率
    pub total_keystrokes: f64,     // 総打鍵数の少なさ
    pub alternating: f64,          // 左右交互打鍵率

    // Bonus Metrics（加算）- その他全て
    pub single_key: f64,           // 単打鍵率
    pub colemak_similarity: f64,   // Colemak類似度
    pub position_cost: f64,        // 位置別コスト
    pub redirect_low: f64,         // リダイレクト少なさ
    pub tsuki_similarity: f64,     // 月配列類似度
    pub roll: f64,                 // ロール率
    pub inroll: f64,               // インロール率
    pub arpeggio: f64,             // アルペジオ率
    pub memorability: f64,         // 覚えやすさ
    pub shift_balance: f64,        // シフトバランス
}

impl Default for EvaluationWeights {
    fn default() -> Self {
        Self {
            // Core（乗算・指数）- 基本5指標に集中
            same_finger: 1.7,        // 同指連続率の低さ
            row_skip: 1.6,           // 段越えの少なさ
            home_position: 1.5,      // ホームポジション率
            total_keystrokes: 1.4,   // 総打鍵数の少なさ
            alternating: 1.3,        // 左右交互打鍵率

            // Bonus（加算）- その他全て
            single_key: 2.0,         // 単打鍵率
            colemak_similarity: 10.0, // Colemak類似度（高比重Bonus）
            position_cost: 8.0,      // 位置別コスト（高比重Bonus）
            redirect_low: 5.0,       // リダイレクト少なさ（高比重Bonus）
            tsuki_similarity: 2.0,   // 月配列類似度
            roll: 6.0,               // ロール率（高比重Bonus）
            inroll: 5.0,             // インロール率（高比重Bonus）
            arpeggio: 5.0,           // アルペジオ率（高比重Bonus）
            memorability: 2.0,       // 覚えやすさ
            shift_balance: 3.0,      // シフトバランス
        }
    }
}

// ============================================================================
// 評価器
// ============================================================================

/// 配列評価器
pub struct Evaluator {
    pub corpus: CorpusStats,
    pub tsuki: TsukiLayout,
    pub weights: EvaluationWeights,
}

impl Evaluator {
    /// 新しい評価器を作成
    pub fn new(corpus: CorpusStats) -> Self {
        Self {
            corpus,
            tsuki: TsukiLayout::new(),
            weights: EvaluationWeights::default(),
        }
    }

    /// カスタム重みで評価器を作成
    pub fn with_weights(corpus: CorpusStats, weights: EvaluationWeights) -> Self {
        Self {
            corpus,
            tsuki: TsukiLayout::new(),
            weights,
        }
    }

    /// 配列を評価してフィットネス値を計算
    pub fn evaluate(&self, layout: &mut Layout) -> f64 {
        let scores = self.compute_scores(layout);
        let fitness = self.compute_fitness(&scores);
        layout.scores = scores;
        layout.fitness = fitness;
        fitness
    }

    /// スコアからフィットネス値を計算
    pub fn compute_fitness(&self, scores: &EvaluationScores) -> f64 {
        let w = &self.weights;

        // Core metrics（乗算）- 基本5指標のみ
        let same_finger_norm = (scores.same_finger / 100.0).max(0.01);
        let row_skip_norm = (scores.row_skip / 100.0).max(0.01);
        let home_norm = (scores.home_position / 100.0).max(0.01);
        let total_keystrokes_norm = (scores.total_keystrokes / 100.0).max(0.01);
        let alternating_norm = (scores.alternating / 100.0).max(0.01);

        let total_core_weight = w.same_finger + w.row_skip + w.home_position
            + w.total_keystrokes + w.alternating;

        let core_product = same_finger_norm.powf(w.same_finger)
            * row_skip_norm.powf(w.row_skip)
            * home_norm.powf(w.home_position)
            * total_keystrokes_norm.powf(w.total_keystrokes)
            * alternating_norm.powf(w.alternating);

        let core_multiplier = core_product.powf(1.0 / total_core_weight) * 100.0;

        // Bonus metrics（加算）- その他全て
        let additive_bonus = scores.single_key * w.single_key
            + scores.colemak_similarity * w.colemak_similarity
            + scores.position_cost * w.position_cost
            + scores.redirect_low * w.redirect_low
            + scores.tsuki_similarity * w.tsuki_similarity
            + scores.roll * w.roll
            + scores.inroll * w.inroll
            + scores.arpeggio * w.arpeggio
            + scores.memorability * w.memorability
            + scores.shift_balance * w.shift_balance;

        let bonus_scale = (w.single_key + w.colemak_similarity + w.position_cost
            + w.redirect_low + w.tsuki_similarity + w.roll
            + w.inroll + w.arpeggio + w.memorability + w.shift_balance) * 100.0;

        // Final: core × (1 + bonus/scale)
        core_multiplier * (1.0 + additive_bonus / bonus_scale)
    }

    /// 詳細スコアを計算
    fn compute_scores(&self, layout: &Layout) -> EvaluationScores {
        let char_map = layout.build_char_map();
        
        let mut total_keystrokes = 0.0;
        let mut home_keystrokes = 0.0;
        let mut shifted_keystrokes = 0.0;
        let mut single_keystrokes = 0.0;
        let mut layer1_keystrokes = 0.0;
        let mut layer2_keystrokes = 0.0;
        let mut row_skips = 0.0;
        let mut same_finger = 0.0;
        let mut alternating = 0.0;
        let mut total_chars = 0.0;
        let mut missing_chars = 0.0;

        // 1-gram統計
        for (&c, &count) in &self.corpus.char_freq {
            let count_f = count as f64;
            if let Some(&pos) = char_map.get(&c) {
                total_chars += count_f;
                total_keystrokes += pos.weight() * count_f;

                if pos.is_home() {
                    home_keystrokes += count_f;
                }

                if pos.layer > 0 {
                    shifted_keystrokes += count_f;
                    if pos.layer == 1 {
                        layer1_keystrokes += count_f;
                    } else if pos.layer == 2 {
                        layer2_keystrokes += count_f;
                    }
                } else {
                    single_keystrokes += count_f;
                }
            } else {
                missing_chars += count_f;
            }
        }

        // 2-gram統計
        let mut bigram_counted = 0.0;
        for (&(c1, c2), &count) in &self.corpus.bigram_freq {
            if let (Some(&pos1), Some(&pos2)) = (char_map.get(&c1), char_map.get(&c2)) {
                let count_f = count as f64;
                bigram_counted += count_f;

                // 同指連続
                if pos1.is_left_hand() == pos2.is_left_hand() && pos1.finger() == pos2.finger() {
                    same_finger += count_f;

                    // 段飛ばし
                    let row_diff = (pos1.row as i32 - pos2.row as i32).abs();
                    if row_diff > 1 {
                        row_skips += count_f;
                    }
                }

                // 左右交互
                if pos1.is_left_hand() != pos2.is_left_hand() {
                    alternating += count_f;
                }
            }
        }

        // 3-gram統計
        let (roll_rate, redirect_low, inroll_rate, arpeggio_rate) = self.calc_trigram_scores(layout, &char_map);

        let corpus_total = total_chars + missing_chars;
        let coverage = if corpus_total > 0.0 { total_chars / corpus_total } else { 0.0 };

        EvaluationScores {
            row_skip: if bigram_counted > 0.0 {
                100.0 * (1.0 - row_skips / bigram_counted) * coverage
            } else { 0.0 },

            home_position: if corpus_total > 0.0 {
                100.0 * home_keystrokes / corpus_total
            } else { 0.0 },

            total_keystrokes: if total_chars > 0.0 {
                let avg_weight = total_keystrokes / total_chars;
                100.0 * (1.0 - (avg_weight - 1.0) / 4.0).max(0.0) * coverage
            } else { 0.0 },

            same_finger: if bigram_counted > 0.0 {
                100.0 * (1.0 - same_finger / bigram_counted) * coverage
            } else { 0.0 },

            single_key: if corpus_total > 0.0 {
                100.0 * single_keystrokes / corpus_total
            } else { 0.0 },

            colemak_similarity: self.calc_colemak_similarity(layout),
            position_cost: self.calc_position_cost(layout, &char_map),
            tsuki_similarity: self.calc_tsuki_similarity(layout),
            memorability: self.calc_memorability(layout),

            alternating: if bigram_counted > 0.0 {
                100.0 * alternating / bigram_counted * coverage
            } else { 0.0 },

            roll: roll_rate,
            redirect_low,
            inroll: inroll_rate,
            arpeggio: arpeggio_rate,

            shift_balance: {
                let total_shifted = layer1_keystrokes + layer2_keystrokes;
                if total_shifted > 0.0 {
                    let ratio = layer1_keystrokes.min(layer2_keystrokes)
                        / layer1_keystrokes.max(layer2_keystrokes);
                    100.0 * ratio
                } else {
                    100.0
                }
            },
        }
    }

    /// 3-gramスコアを計算
    fn calc_trigram_scores(&self, layout: &Layout, char_map: &HashMap<char, KeyPos>) -> (f64, f64, f64, f64) {
        let mut roll_count = 0.0;
        let mut redirect_count = 0.0;
        let mut inroll_count = 0.0;
        let mut arpeggio_count = 0.0;
        let mut trigram_counted = 0.0;

        for (&(c1, c2, c3), &count) in &self.corpus.trigram_freq {
            if let (Some(&pos1), Some(&pos2), Some(&pos3)) = (
                char_map.get(&c1),
                char_map.get(&c2),
                char_map.get(&c3),
            ) {
                let count_f = count as f64;
                trigram_counted += count_f;

                let hand1 = pos1.is_left_hand();
                let hand2 = pos2.is_left_hand();
                let hand3 = pos3.is_left_hand();
                let finger1 = pos1.finger();
                let finger2 = pos2.finger();
                let finger3 = pos3.finger();

                // 同じ手で3打鍵
                if hand1 == hand2 && hand2 == hand3 {
                    let dir1 = finger2 as i32 - finger1 as i32;
                    let dir2 = finger3 as i32 - finger2 as i32;

                    if finger1 != finger2 && finger2 != finger3 && finger1 != finger3 {
                        if (dir1 > 0 && dir2 > 0) || (dir1 < 0 && dir2 < 0) {
                            // ロール（同方向）
                            roll_count += count_f;

                            // インロール（外→内）
                            if dir1 > 0 {
                                inroll_count += count_f;
                            }

                            // アルペジオ（隣接指）
                            if dir1.abs() == 1 && dir2.abs() == 1 {
                                arpeggio_count += count_f;
                            }
                        } else if (dir1 > 0 && dir2 < 0) || (dir1 < 0 && dir2 > 0) {
                            // リダイレクト（方向転換）
                            redirect_count += count_f;
                        }
                    }
                }
            }
        }

        let coverage = if trigram_counted > 0.0 { 1.0 } else { 0.0 };

        (
            if trigram_counted > 0.0 { 100.0 * roll_count / trigram_counted * coverage } else { 0.0 },
            if trigram_counted > 0.0 { 100.0 * (1.0 - redirect_count / trigram_counted) * coverage } else { 0.0 },
            if trigram_counted > 0.0 { 100.0 * inroll_count / trigram_counted * coverage } else { 0.0 },
            if trigram_counted > 0.0 { 100.0 * arpeggio_count / trigram_counted * coverage } else { 0.0 },
        )
    }

    /// 位置別コスト評価（幾何平均方式）
    /// 高頻度文字を低コスト位置に配置できているかを評価
    /// 乗算ベースなので高コスト位置への配置がより強くペナルティされる
    fn calc_position_cost(&self, layout: &Layout, char_map: &HashMap<char, KeyPos>) -> f64 {
        // ベースコスト（No Layer） - 1.0が最良
        const BASE_COST: [[f64; COLS]; ROWS] = [
            [4.0, 2.0, 2.0, 2.0, 3.0,  4.0, 2.0, 2.0, 2.0, 4.0],  // 上段
            [2.0, 1.0, 1.0, 1.0, 2.0,  2.0, 1.0, 1.0, 1.0, 2.0],  // 中段
            [4.0, 3.0, 2.0, 2.0, 4.0,  3.0, 2.0, 2.0, 2.0, 4.0],  // 下段
        ];

        // 幾何平均: exp(Σ(freq × ln(cost)) / Σfreq)
        // ln(1/cost)を使うと低コスト=高スコアになる
        let mut log_sum = 0.0;
        let mut total_freq = 0.0;

        // 1. 通常文字の評価
        for (&c, &count) in &self.corpus.char_freq {
            if let Some(&pos) = char_map.get(&c) {
                let base = BASE_COST[pos.row][pos.col];

                // Layer 1,2はLayer 0よりわずかにペナルティ（シフト操作コスト）
                let layer_penalty = if pos.layer == 0 { 1.0 } else { 1.05 };
                let mut multiplier = layer_penalty;

                if pos.layer == 1 {
                    // Layer 1（☆シフト）
                    multiplier = 3.0 * layer_penalty;

                    // Ver: ☆の上下段（col=7, row!=1）
                    if pos.col == 7 && pos.row != 1 {
                        multiplier += 27.0;
                    }
                    // Out: ☆より小指側（col >= 8）
                    if pos.col >= 8 {
                        multiplier += 9.0;
                    }
                } else if pos.layer == 2 {
                    // Layer 2（★シフト）
                    multiplier = 3.0 * layer_penalty;

                    // Ver: ★の上下段（col=2, row!=1）
                    if pos.col == 2 && pos.row != 1 {
                        multiplier += 27.0;
                    }
                    // Out: ★より小指側（col <= 1）
                    if pos.col <= 1 {
                        multiplier += 9.0;
                    }
                }

                let cost = base * multiplier;
                let freq = count as f64;

                // ln(1/cost) = -ln(cost) を頻度で重み付け
                log_sum += freq * (-(cost as f64).ln());
                total_freq += freq;
            }
        }

        // 2. 空白文字のペナルティ（乗算で反映）
        for layer in 0..NUM_LAYERS {
            for row in 0..ROWS {
                for col in 0..COLS {
                    let c = layout.layers[layer][row][col];

                    if c != '　' || crate::layout::Layout::is_fixed_position(layer, row, col) {
                        continue;
                    }

                    let is_ver = (layer == 1 && col == 7 && row != 1) ||
                                 (layer == 2 && col == 2 && row != 1);
                    let is_out = (layer == 1 && col >= 8) ||
                                 (layer == 2 && col <= 1);

                    // 空白位置のコスト（乗算に反映）
                    let blank_cost: f64 = if is_ver {
                        0.5  // Ver位置の空白はボーナス（コスト<1）
                    } else if is_out {
                        2.0  // Out位置の空白は許容
                    } else {
                        50.0 // その他の空白は強いペナルティ
                    };

                    // 空白1個あたりの重み（頻度相当）
                    let blank_weight = total_freq * 0.01; // 全体の1%相当
                    log_sum += blank_weight * (-blank_cost.ln());
                    total_freq += blank_weight;
                }
            }
        }

        if total_freq == 0.0 {
            return 0.0;
        }

        // 幾何平均のスコア = exp(log_sum / total_freq)
        // これは 1/幾何平均(costs) に相当（低コスト=高スコア）
        let geo_score = (log_sum / total_freq).exp();

        // 最大スコア: 1/1.0 = 1.0（全て最良位置）
        // 最小スコア: 1/120 ≈ 0.008（全て最悪位置）
        // 0-100%にスケーリング
        let normalized = geo_score.min(1.0);
        normalized * 100.0
    }
    
    /// Colemak類似度を計算
    /// - Layer 0: 100%の重み
    /// - Layer 1, 2: 80%の重み
    fn calc_colemak_similarity(&self, layout: &Layout) -> f64 {
        let mut total_score = 0.0;
        let mut max_score = 0.0;

        let mut phoneme_pos: HashMap<&str, (usize, usize)> = HashMap::new();
        for &(phoneme, row, col) in COLEMAK_POSITIONS {
            phoneme_pos.insert(phoneme, (row, col));
        }

        for layer in 0..NUM_LAYERS {
            let layer_weight = if layer == 0 { 1.0 } else { 0.3 };

            for row in 0..ROWS {
                for col in 0..COLS {
                    let c = layout.layers[layer][row][col];
                    if c == '☆' || c == '★' || c == '、' || c == '。' || c == '　' || c == '\0' || c == '゛' || c == '゜' {
                        continue;
                    }

                    let (consonant, vowel) = romaji_phonemes(c);

                    let char_max_score = match (consonant, vowel) {
                        (Some(_), Some(_)) => 2.0 * layer_weight,
                        (Some(_), None) | (None, Some(_)) => 1.0 * layer_weight,
                        (None, None) => 0.0,
                    };
                    max_score += char_max_score;

                    if char_max_score == 0.0 {
                        continue;
                    }

                    // 子音チェック
                    if let Some(cons) = consonant {
                        if let Some(&(exp_row, exp_col)) = phoneme_pos.get(cons) {
                            if row == exp_row && col == exp_col {
                                total_score += 1.0 * layer_weight;
                            } else if row == exp_row {
                                total_score += 0.5 * layer_weight;
                            } else if (col < 5 && exp_col < 5) || (col >= 5 && exp_col >= 5) {
                                total_score += 0.25 * layer_weight;
                            }
                        }
                    }

                    // 母音チェック
                    if let Some(vow) = vowel {
                        if let Some(&(exp_row, exp_col)) = phoneme_pos.get(vow) {
                            if row == exp_row && col == exp_col {
                                total_score += 1.0 * layer_weight;
                            } else if row == exp_row {
                                total_score += 0.5 * layer_weight;
                            } else if (col < 5 && exp_col < 5) || (col >= 5 && exp_col >= 5) {
                                total_score += 0.25 * layer_weight;
                            }
                        }
                    }
                }
            }
        }

        if max_score > 0.0 {
            100.0 * total_score / max_score
        } else {
            0.0
        }
    }

    /// 月配列類似度を計算
    /// - GA Layer 0 → 月 表面 (layer 0)
    /// - GA Layer 1, 2 → 月 裏面 (layer 1)
    fn calc_tsuki_similarity(&self, layout: &Layout) -> f64 {
        let mut matches = 0;
        let mut total = 0;

        for ga_layer in 0..NUM_LAYERS {
            let expected_tsuki_layer = if ga_layer == 0 { 0 } else { 1 };

            for row in 0..ROWS {
                for col in 0..COLS {
                    let c = layout.layers[ga_layer][row][col];
                    if c == '☆' || c == '★' || c == '、' || c == '。' || c == '　' || c == '\0' || c == '゛' || c == '゜' {
                        continue;
                    }

                    if let Some(&tsuki_pos) = self.tsuki.char_positions.get(&c) {
                        if tsuki_pos.layer == expected_tsuki_layer {
                            total += 1;
                            if row == tsuki_pos.row && col == tsuki_pos.col {
                                matches += 1;
                            }
                        }
                    }
                }
            }
        }

        if total > 0 {
            100.0 * matches as f64 / total as f64
        } else {
            0.0
        }
    }

    /// 覚えやすさを計算（母音/子音の一貫性）
    fn calc_memorability(&self, layout: &Layout) -> f64 {
        let vowels: HashSet<char> = ['あ', 'い', 'う', 'え', 'お', 'ぁ', 'ぃ', 'ぅ', 'ぇ', 'ぉ']
            .iter()
            .copied()
            .collect();

        let mut consistent_positions = 0;
        let mut total_positions = 0;

        for row in 0..ROWS {
            for col in 0..COLS {
                if Layout::is_fixed_position(0, row, col) {
                    continue;
                }

                let c0 = layout.layers[0][row][col];
                if c0 == '　' || c0 == '\0' {
                    continue;
                }

                let is_vowel_0 = vowels.contains(&c0);
                let mut layer_consistent = 0;
                let mut layer_checked = 0;

                for layer in 1..NUM_LAYERS {
                    if Layout::is_fixed_position(layer, row, col) {
                        continue;
                    }

                    let c_layer = layout.layers[layer][row][col];
                    if c_layer == '　' || c_layer == '\0' {
                        continue;
                    }

                    layer_checked += 1;
                    let is_vowel_layer = vowels.contains(&c_layer);

                    if is_vowel_0 == is_vowel_layer {
                        layer_consistent += 1;
                    }
                }

                if layer_checked > 0 {
                    total_positions += layer_checked;
                    consistent_positions += layer_consistent;
                }
            }
        }

        if total_positions > 0 {
            100.0 * consistent_positions as f64 / total_positions as f64
        } else {
            0.0
        }
    }
}
