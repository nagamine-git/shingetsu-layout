//! 遺伝的アルゴリズムモジュール
//! 
//! 配列最適化のための遺伝的アルゴリズムを実装。

use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;

use crate::corpus::CorpusStats;
use crate::evaluation::{EvaluationWeights, Evaluator};
use crate::layout::{Layout, NUM_LAYERS, ROWS, cols_for_row, HIRAGANA_FREQ_DEFAULT};

/// 遺伝的アルゴリズムの設定
#[derive(Debug, Clone)]
pub struct GaConfig {
    /// 集団サイズ
    pub population_size: usize,
    /// 世代数
    pub generations: usize,
    /// 突然変異率
    pub mutation_rate: f64,
    /// エリート保持数
    pub elite_count: usize,
    /// 乱数シード
    pub seed: u64,
}

impl Default for GaConfig {
    fn default() -> Self {
        Self {
            population_size: 500,
            generations: 1000,
            mutation_rate: 0.15,
            elite_count: 10,
            seed: 42,
        }
    }
}

/// 遺伝的アルゴリズムの実行結果
#[derive(Debug, Clone)]
pub struct GaResult {
    /// 最良の配列
    pub best_layout: Layout,
    /// 最良のフィットネス値
    pub best_fitness: f64,
    /// 世代ごとのフィットネス履歴
    pub fitness_history: Vec<f64>,
    /// 最終世代
    pub final_generation: usize,
}

/// 遺伝的アルゴリズム実行器
pub struct GeneticAlgorithm {
    config: GaConfig,
    evaluator: Evaluator,
    rng: ChaCha8Rng,
    /// コーパスから取得したひらがな頻度順リスト（String型）
    hiragana_chars: Vec<String>,
}

impl GeneticAlgorithm {
    /// 新しいGAインスタンスを作成
    pub fn new(corpus: CorpusStats, config: GaConfig) -> Self {
        let rng = ChaCha8Rng::seed_from_u64(config.seed);
        // コーパスから頻度順リストを取得し、欠落文字をデフォルトリストで補完
        let corpus_chars: Vec<&str> = corpus.hiragana_by_freq.iter().map(|c| {
            // charを&strに変換するためにマッチング
            match *c {
                'い' => "い", 'う' => "う", 'ん' => "ん", 'し' => "し", 'か' => "か",
                'の' => "の", 'と' => "と", 'た' => "た", 'て' => "て", 'く' => "く",
                'な' => "な", 'に' => "に", 'き' => "き", 'は' => "は", 'こ' => "こ",
                'る' => "る", 'が' => "が", 'で' => "で", 'っ' => "っ", 'す' => "す",
                'ま' => "ま", 'じ' => "じ", 'り' => "り", 'も' => "も", 'つ' => "つ",
                'お' => "お", 'ら' => "ら", 'を' => "を", 'さ' => "さ", 'あ' => "あ",
                'れ' => "れ", 'だ' => "だ", 'ち' => "ち", 'せ' => "せ", 'け' => "け",
                'ー' => "ー", 'よ' => "よ", 'ど' => "ど", 'そ' => "そ", 'え' => "え",
                'わ' => "わ", 'み' => "み", 'め' => "め", 'ひ' => "ひ", 'ば' => "ば",
                'や' => "や", 'ろ' => "ろ", 'ほ' => "ほ", 'ふ' => "ふ", 'ぶ' => "ぶ",
                'ね' => "ね", 'ご' => "ご", 'ぎ' => "ぎ", 'げ' => "げ", 'む' => "む",
                'び' => "び", 'ざ' => "ざ", 'ぐ' => "ぐ", 'ぜ' => "ぜ", 'へ' => "へ",
                'べ' => "べ", 'ゆ' => "ゆ", 'ぼ' => "ぼ", 'ぷ' => "ぷ", 'ぞ' => "ぞ",
                'ぱ' => "ぱ", 'ぴ' => "ぴ", 'づ' => "づ", 'ぺ' => "ぺ", 'ぬ' => "ぬ",
                'ぽ' => "ぽ", 'ヴ' => "ヴ", 'ぢ' => "ぢ", 'ず' => "ず",
                'ぁ' => "ぁ", 'ぃ' => "ぃ", 'ぅ' => "ぅ", 'ぇ' => "ぇ", 'ぉ' => "ぉ",
                _ => "　",
            }
        }).collect();
        let hiragana_chars = Self::ensure_complete_charset(&corpus_chars);
        Self {
            evaluator: Evaluator::new(corpus),
            config,
            rng,
            hiragana_chars,
        }
    }

    /// カスタム重みでGAインスタンスを作成
    pub fn with_weights(corpus: CorpusStats, config: GaConfig, weights: EvaluationWeights) -> Self {
        let rng = ChaCha8Rng::seed_from_u64(config.seed);
        // コーパスから頻度順リストを取得し、欠落文字をデフォルトリストで補完
        let corpus_chars: Vec<&str> = corpus.hiragana_by_freq.iter().map(|c| {
            match *c {
                'い' => "い", 'う' => "う", 'ん' => "ん", 'し' => "し", 'か' => "か",
                'の' => "の", 'と' => "と", 'た' => "た", 'て' => "て", 'く' => "く",
                'な' => "な", 'に' => "に", 'き' => "き", 'は' => "は", 'こ' => "こ",
                'る' => "る", 'が' => "が", 'で' => "で", 'っ' => "っ", 'す' => "す",
                'ま' => "ま", 'じ' => "じ", 'り' => "り", 'も' => "も", 'つ' => "つ",
                'お' => "お", 'ら' => "ら", 'を' => "を", 'さ' => "さ", 'あ' => "あ",
                'れ' => "れ", 'だ' => "だ", 'ち' => "ち", 'せ' => "せ", 'け' => "け",
                'ー' => "ー", 'よ' => "よ", 'ど' => "ど", 'そ' => "そ", 'え' => "え",
                'わ' => "わ", 'み' => "み", 'め' => "め", 'ひ' => "ひ", 'ば' => "ば",
                'や' => "や", 'ろ' => "ろ", 'ほ' => "ほ", 'ふ' => "ふ", 'ぶ' => "ぶ",
                'ね' => "ね", 'ご' => "ご", 'ぎ' => "ぎ", 'げ' => "げ", 'む' => "む",
                'び' => "び", 'ざ' => "ざ", 'ぐ' => "ぐ", 'ぜ' => "ぜ", 'へ' => "へ",
                'べ' => "べ", 'ゆ' => "ゆ", 'ぼ' => "ぼ", 'ぷ' => "ぷ", 'ぞ' => "ぞ",
                'ぱ' => "ぱ", 'ぴ' => "ぴ", 'づ' => "づ", 'ぺ' => "ぺ", 'ぬ' => "ぬ",
                'ぽ' => "ぽ", 'ヴ' => "ヴ", 'ぢ' => "ぢ", 'ず' => "ず",
                'ぁ' => "ぁ", 'ぃ' => "ぃ", 'ぅ' => "ぅ", 'ぇ' => "ぇ", 'ぉ' => "ぉ",
                _ => "　",
            }
        }).collect();
        let hiragana_chars = Self::ensure_complete_charset(&corpus_chars);
        Self {
            evaluator: Evaluator::with_weights(corpus, weights),
            config,
            rng,
            hiragana_chars,
        }
    }

    /// コーパスの文字リストが全文字を含むことを保証
    /// 欠落している文字はHIRAGANA_FREQ_DEFAULTから補完（末尾に追加）
    /// 固定文字（ー, ; など）は除外
    fn ensure_complete_charset(corpus_chars: &[&str]) -> Vec<String> {
        use std::collections::HashSet;

        // 固定文字（配置対象外）
        let fixed_chars: HashSet<&str> = ["★", "☆", "◆", "、", "。", "ー", "・", ";"].iter().copied().collect();

        // 固定文字を除外してコピー
        let mut result: Vec<String> = corpus_chars
            .iter()
            .filter(|s| !fixed_chars.contains(**s))
            .map(|s| s.to_string())
            .collect();
        let existing: HashSet<&str> = corpus_chars.iter().copied().collect();

        // デフォルトリストから欠落文字を探して末尾に追加（固定文字は除外）
        for &c in HIRAGANA_FREQ_DEFAULT {
            if !existing.contains(c) && !fixed_chars.contains(c) {
                result.push(c.to_string());
            }
        }

        result
    }

    /// 最適化を実行
    pub fn run(&mut self) -> GaResult {
        self.run_with_callback(|_, _, _| {})
    }

    /// コールバック付きで最適化を実行
    /// 
    /// コールバック: `fn(generation: usize, best_fitness: f64, best_layout: &Layout)`
    pub fn run_with_callback<F>(&mut self, mut callback: F) -> GaResult
    where
        F: FnMut(usize, f64, &Layout),
    {
        // 初期集団の生成（1つは改善版カスタムレイアウト、残りはランダム）
        let mut population: Vec<Layout> = Vec::with_capacity(self.config.population_size);
        
        // 最初の1つは改善版カスタムレイアウト
        let mut custom_layout = Layout::improved_custom();
        self.repair_layout(&mut custom_layout);
        self.evaluator.evaluate(&mut custom_layout);
        population.push(custom_layout);
        
        // 残りはランダム生成
        for _ in 1..self.config.population_size {
            let chars_refs: Vec<&str> = self.hiragana_chars.iter().map(|s| s.as_str()).collect();
            let mut layout = Layout::random_with_chars(&mut self.rng, &chars_refs);
            self.repair_layout(&mut layout);
            self.evaluator.evaluate(&mut layout);
            population.push(layout);
        }

        // フィットネスでソート（降順）
        population.sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap());

        let mut best_layout = population[0].clone();
        let mut best_fitness = population[0].fitness;
        let mut fitness_history = Vec::with_capacity(self.config.generations);

        fitness_history.push(best_fitness);
        callback(0, best_fitness, &best_layout);

        // 世代ループ
        for gen in 1..=self.config.generations {
            // 選択・交叉・突然変異
            let mut new_population = Vec::with_capacity(self.config.population_size);

            // エリート保持
            for i in 0..self.config.elite_count.min(population.len()) {
                new_population.push(population[i].clone());
            }

            // 残りを生成
            while new_population.len() < self.config.population_size {
                // トーナメント選択
                let parent1 = self.tournament_select(&population);
                let parent2 = self.tournament_select(&population);

                // 交叉
                let mut child = self.crossover(&parent1, &parent2);

                // 突然変異
                if self.rng.gen::<f64>() < self.config.mutation_rate {
                    self.mutate(&mut child);
                    self.repair_layout(&mut child);  // 重複除去
                }

                // 評価
                self.evaluator.evaluate(&mut child);
                new_population.push(child);
            }

            // フィットネスでソート
            new_population.sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap());
            population = new_population;

            // 最良更新
            if population[0].fitness > best_fitness {
                best_fitness = population[0].fitness;
                best_layout = population[0].clone();
            }

            fitness_history.push(best_fitness);
            callback(gen, best_fitness, &best_layout);
        }

        // 最終結果の重複チェックと再評価
        self.repair_layout(&mut best_layout);
        self.evaluator.evaluate(&mut best_layout);
        let final_fitness = best_layout.fitness;

        GaResult {
            best_layout,
            best_fitness: final_fitness,
            fitness_history,
            final_generation: self.config.generations,
        }
    }

    /// トーナメント選択
    fn tournament_select(&mut self, population: &[Layout]) -> Layout {
        let tournament_size = 5;
        let mut best: Option<&Layout> = None;

        for _ in 0..tournament_size {
            let idx = self.rng.gen_range(0..population.len());
            let candidate = &population[idx];
            if best.is_none() || candidate.fitness > best.unwrap().fitness {
                best = Some(candidate);
            }
        }

        best.unwrap().clone()
    }

    /// 交叉（一様交叉）
    fn crossover(&mut self, parent1: &Layout, parent2: &Layout) -> Layout {
        let mut child = Layout::default();

        // 各ポジションで親をランダムに選択
        for layer in 0..NUM_LAYERS {
            for row in 0..ROWS {
                let cols = cols_for_row(row);
                for col in 0..cols {
                    if Layout::is_fixed_position(layer, row, col) {
                        child.layers[layer][row][col] = parent1.layers[layer][row][col].clone();
                    } else if Layout::is_blank_position(layer, row, col) {
                        child.layers[layer][row][col] = "　".to_string();  // 空白位置は常に空白
                    } else if self.rng.gen::<bool>() {
                        child.layers[layer][row][col] = parent1.layers[layer][row][col].clone();
                    } else {
                        child.layers[layer][row][col] = parent2.layers[layer][row][col].clone();
                    }
                }
            }
        }

        // 重複を修正
        self.repair_layout(&mut child);
        child
    }

    /// 突然変異（2つの位置をスワップ）
    fn mutate(&mut self, layout: &mut Layout) {
        // ランダムな非固定位置を2つ選択してスワップ
        // 空白位置（シフト制限）もスワップから除外
        let mut positions: Vec<(usize, usize, usize)> = Vec::new();
        for l in 0..NUM_LAYERS {
            for r in 0..ROWS {
                let cols = cols_for_row(r);
                for c in 0..cols {
                    // 固定位置と空白位置はスワップ禁止
                    if !Layout::is_fixed_position(l, r, c) && !Layout::is_blank_position(l, r, c) {
                        positions.push((l, r, c));
                    }
                }
            }
        }

        if positions.len() >= 2 {
            let idx1 = self.rng.gen_range(0..positions.len());
            let idx2 = self.rng.gen_range(0..positions.len());

            if idx1 != idx2 {
                let (l1, r1, c1) = positions[idx1];
                let (l2, r2, c2) = positions[idx2];

                let tmp = layout.layers[l1][r1][c1].clone();
                layout.layers[l1][r1][c1] = layout.layers[l2][r2][c2].clone();
                layout.layers[l2][r2][c2] = tmp;
            }
        }
    }

    /// 配列の重複を修正
    fn repair_layout(&mut self, layout: &mut Layout) {
        use std::collections::HashSet;

        // まず、固定位置を強制的に設定（交叉で壊れた場合に備える）
        // Layer 0: シフトキー（★,☆,◆）と句読点、長音符
        layout.layers[0][1][2] = "★".to_string();   // 左中指 → Layer 2
        layout.layers[0][1][7] = "☆".to_string();   // 右中指 → Layer 1
        layout.layers[0][2][9] = "◆".to_string();   // 右小指 → Layer 3
        layout.layers[0][2][7] = "、".to_string();
        layout.layers[0][2][8] = "。".to_string();
        layout.layers[0][1][10] = "ー".to_string(); // 長音符

        // Layer 1: 中黒
        layout.layers[1][1][10] = "・".to_string();

        // Layer 3: セミコロン
        layout.layers[3][1][10] = ";".to_string();

        // 空白位置を強制設定（シフトキーと同手の制限位置）
        for layer in 0..NUM_LAYERS {
            for row in 0..ROWS {
                let cols = cols_for_row(row);
                for col in 0..cols {
                    if Layout::is_blank_position(layer, row, col) {
                        layout.layers[layer][row][col] = "　".to_string();
                    }
                }
            }
        }

        // 固定文字のセット（重複検出でスキップ）
        let fixed_chars: HashSet<&str> = ["★", "☆", "◆", "、", "。", "ー", "・", ";"].iter().copied().collect();

        let mut seen: HashSet<String> = HashSet::new();
        let mut missing: Vec<String> = Vec::new();
        let mut duplicates: Vec<(usize, usize, usize)> = Vec::new();

        // 重複を検出
        for layer in 0..NUM_LAYERS {
            for row in 0..ROWS {
                let cols = cols_for_row(row);
                for col in 0..cols {
                    // 固定位置と空白位置はスキップ
                    if Layout::is_fixed_position(layer, row, col) {
                        continue;
                    }
                    if Layout::is_blank_position(layer, row, col) {
                        continue;
                    }

                    let s = &layout.layers[layer][row][col];

                    // 固定文字が配置可能位置に入り込んでいたら重複扱い
                    if fixed_chars.contains(s.as_str()) {
                        duplicates.push((layer, row, col));
                        continue;
                    }

                    // 空白位置以外の全角スペースや重複文字を検出
                    if s == "　" || s.is_empty() || seen.contains(s) {
                        duplicates.push((layer, row, col));
                    } else {
                        seen.insert(s.clone());
                    }
                }
            }
        }

        // 欠落文字を検出（コーパスから取得した頻度順リストを使用）
        for c in &self.hiragana_chars {
            // 固定文字はスキップ
            if fixed_chars.contains(c.as_str()) {
                continue;
            }
            if !seen.contains(c) {
                missing.push(c.clone());
            }
        }

        // 重複位置に欠落文字を配置
        missing.shuffle(&mut self.rng);
        for (i, (layer, row, col)) in duplicates.iter().enumerate() {
            if let Some(replacement) = missing.get(i) {
                layout.layers[*layer][*row][*col] = replacement.clone();
            }
            // 欠落文字が足りない場合でも警告を出さない（通常は発生しないはず）
        }
    }
}

/// 並列マルチラン実行
pub fn run_multi(
    corpus: CorpusStats,
    config: GaConfig,
    weights: EvaluationWeights,
    num_runs: usize,
) -> Vec<GaResult> {
    let seeds: Vec<u64> = (0..num_runs)
        .map(|_| rand::thread_rng().gen::<u64>())
        .collect();

    seeds
        .into_par_iter()
        .map(|seed| {
            let mut run_config = config.clone();
            run_config.seed = seed;
            let mut ga = GeneticAlgorithm::with_weights(
                corpus.clone(),
                run_config,
                weights.clone(),
            );
            ga.run()
        })
        .collect()
}

/// マルチラン実行（途中結果を外部ベクタに追加）
pub fn run_multi_with_storage(
    corpus: CorpusStats,
    config: GaConfig,
    weights: EvaluationWeights,
    num_runs: usize,
    completed_storage: std::sync::Arc<std::sync::Mutex<Vec<GaResult>>>,
) -> Vec<GaResult> {
    
    let seeds: Vec<u64> = (0..num_runs)
        .map(|_| rand::thread_rng().gen::<u64>())
        .collect();

    seeds
        .into_par_iter()
        .map(|seed| {
            let mut run_config = config.clone();
            run_config.seed = seed;
            let mut ga = GeneticAlgorithm::with_weights(
                corpus.clone(),
                run_config,
                weights.clone(),
            );
            let result = ga.run();
            
            // 完了した結果を外部ストレージに追加
            {
                let mut storage = completed_storage.lock().unwrap();
                storage.push(result.clone());
            }
            
            result
        })
        .collect()
}

/// マルチラン結果のサマリー
pub fn summarize_results(results: &[GaResult]) -> (f64, f64, f64, f64, &GaResult) {
    let fitnesses: Vec<f64> = results.iter().map(|r| r.best_fitness).collect();
    let n = fitnesses.len() as f64;

    let mean = fitnesses.iter().sum::<f64>() / n;
    let variance = fitnesses.iter().map(|f| (f - mean).powi(2)).sum::<f64>() / n;
    let stddev = variance.sqrt();
    let min = fitnesses.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = fitnesses.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    let best_idx = results
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.best_fitness.partial_cmp(&b.best_fitness).unwrap())
        .map(|(i, _)| i)
        .unwrap_or(0);

    (mean, stddev, min, max, &results[best_idx])
}
