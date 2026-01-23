//! かな配列遺伝的アルゴリズム最適化ツール
//!
//! 日本語かな配列を遺伝的アルゴリズムで最適化する。

mod corpus;
mod evaluation;
mod ga;
mod layout;
mod tui;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};

use crate::corpus::CorpusStats;
use crate::evaluation::EvaluationWeights;
use crate::ga::{GaConfig, GeneticAlgorithm};
use crate::layout::Layout;

/// かな配列遺伝的アルゴリズム最適化ツール
#[derive(Parser, Debug, Clone)]
#[command(name = "kana_layout_optimizer")]
#[command(about = "日本語かな配列を遺伝的アルゴリズムで最適化")]
struct Args {
    /// 1-gramファイルパス
    #[arg(long)]
    gram1: Option<PathBuf>,

    /// 2-gramファイルパス
    #[arg(long)]
    gram2: Option<PathBuf>,

    /// 3-gramファイルパス
    #[arg(long)]
    gram3: Option<PathBuf>,

    /// 4-gramファイルパス
    #[arg(long)]
    gram4: Option<PathBuf>,

    /// コーパスファイルパス（N-gram未指定時のフォールバック）
    #[arg(short, long, default_value = "corpus.txt")]
    corpus: PathBuf,

    /// 集団サイズ
    #[arg(short, long, default_value_t = 500)]
    population: usize,

    /// 世代数
    #[arg(short, long, default_value_t = 1000)]
    generations: usize,

    /// 突然変異率
    #[arg(short, long, default_value_t = 0.25)]
    mutation_rate: f64,

    /// エリート保持数
    #[arg(short, long, default_value_t = 10)]
    elite: usize,

    /// 乱数シード
    #[arg(short, long, default_value_t = 42)]
    seed: u64,

    /// 並列実行数（0=単一実行）
    #[arg(long, default_value_t = 0)]
    multi_run: usize,

    /// 出力ファイルパス
    #[arg(short, long, default_value = "best_layout.json")]
    output: PathBuf,

    /// TUIモード（リアルタイム可視化）
    #[arg(long, default_value_t = false)]
    tui: bool,

    // ========================================
    // 評価重みオプション
    // ========================================
    /// Weight: 同指連続率の低さ（SFB排除45%）
    #[arg(long, default_value_t = 2.2)]
    w_same_finger: f64,

    /// Weight: 段越えの少なさ
    #[arg(long, default_value_t = 1.55)]
    w_row_skip: f64,

    /// Weight: ホームポジション率
    #[arg(long, default_value_t = 1.3)]
    w_home_position: f64,

    /// Weight: 総打鍵コスト
    #[arg(long, default_value_t = 1.05)]
    w_total_keystrokes: f64,

    /// Weight: 左右交互打鍵率（統計的交互打鍵25%）
    #[arg(long, default_value_t = 1.1)]
    w_alternating: f64,

    /// Weight: 単打鍵率
    #[arg(long, default_value_t = 0.7)]
    w_single_key: f64,

    /// Weight: Colemak類似度
    #[arg(long, default_value_t = 0.6)]
    w_colemak_similarity: f64,

    /// Weight: 位置別コスト（高頻度文字を低コスト位置に）
    #[arg(long, default_value_t = 1.3)]
    w_position_cost: f64,

    /// Weight: リダイレクト少
    #[arg(long, default_value_t = 5.0)]
    w_redirect_low: f64,

    /// Weight: 月配列類似度
    #[arg(long, default_value_t = 4.0)]
    w_tsuki_similarity: f64,

    /// Weight: ロール率（アルペジオ調和15%）
    #[arg(long, default_value_t = 6.0)]
    w_roll: f64,

    /// Weight: インロール率（内向きロール優遇）
    #[arg(long, default_value_t = 6.0)]
    w_inroll: f64,

    /// Weight: アルペジオ率（片手連打の質）
    #[arg(long, default_value_t = 6.0)]
    w_arpeggio: f64,

    /// Weight: 覚えやすさ
    #[arg(long, default_value_t = 2.0)]
    w_memorability: f64,

    /// Weight: シフトバランス
    #[arg(long, default_value_t = 3.0)]
    w_shift_balance: f64,
}

impl From<&Args> for EvaluationWeights {
    fn from(args: &Args) -> Self {
        Self {
            same_finger: args.w_same_finger,
            row_skip: args.w_row_skip,
            home_position: args.w_home_position,
            total_keystrokes: args.w_total_keystrokes,
            alternating: args.w_alternating,
            single_key: args.w_single_key,
            colemak_similarity: args.w_colemak_similarity,
            position_cost: args.w_position_cost,
            redirect_low: args.w_redirect_low,
            tsuki_similarity: args.w_tsuki_similarity,
            roll: args.w_roll,
            inroll: args.w_inroll,
            arpeggio: args.w_arpeggio,
            memorability: args.w_memorability,
            shift_balance: args.w_shift_balance,
        }
    }
}

fn main() {
    let args = Args::parse();

    println!("=== かな配列遺伝的アルゴリズム最適化 ===\n");

    // コーパス読み込み
    let corpus = load_corpus(&args);
    println!("{}\n", corpus.summary());

    // 設定
    let config = GaConfig {
        population_size: args.population,
        generations: args.generations,
        mutation_rate: args.mutation_rate,
        elite_count: args.elite,
        seed: args.seed,
    };

    let weights = EvaluationWeights::from(&args);

    println!("GA設定:");
    println!("  集団サイズ: {}", config.population_size);
    println!("  世代数: {}", config.generations);
    println!("  突然変異率: {}", config.mutation_rate);
    println!("  エリート保持: {}", config.elite_count);
    println!("  シード: {}", config.seed);
    println!();

    if args.multi_run > 0 {
        // マルチラン実行
        if args.tui && atty::is(atty::Stream::Stdout) {
            run_multi_with_tui(&corpus, config, weights, args.multi_run, &args.output);
        } else {
            run_multi(&corpus, config, weights, args.multi_run, &args.output);
        }
    } else if args.tui && atty::is(atty::Stream::Stdout) {
        // TUIモード（単一実行）
        run_with_tui(&corpus, config, weights, &args.output);
    } else {
        // 通常実行（プログレスバー）
        run_single(&corpus, config, weights, &args.output);
    }
}

/// コーパスを読み込む
fn load_corpus(args: &Args) -> CorpusStats {
    // N-gramファイル優先
    if args.gram1.is_some() || args.gram2.is_some() || args.gram3.is_some() || args.gram4.is_some() {
        println!("N-gramファイルから読み込み中...");
        match CorpusStats::from_ngram_files(
            args.gram1.as_deref(),
            args.gram2.as_deref(),
            args.gram3.as_deref(),
            args.gram4.as_deref(),
        ) {
            Ok(stats) => return stats,
            Err(e) => {
                eprintln!("N-gramファイル読み込みエラー: {}", e);
                eprintln!("コーパスファイルにフォールバック...");
            }
        }
    }

    // コーパスファイルから
    println!("コーパスファイルから読み込み中: {:?}", args.corpus);
    match CorpusStats::from_text_file(&args.corpus) {
        Ok(stats) => stats,
        Err(e) => {
            eprintln!("コーパス読み込みエラー: {}", e);
            eprintln!("空のコーパスで続行...");
            CorpusStats::new()
        }
    }
}

/// 単一実行（プログレスバー）
fn run_single(corpus: &CorpusStats, config: GaConfig, weights: EvaluationWeights, output: &PathBuf) {
    let mut ga = GeneticAlgorithm::with_weights(corpus.clone(), config.clone(), weights.clone());

    // プログレスバー
    let pb = ProgressBar::new(config.generations as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} (Gen) | Best: {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );

    let result = ga.run_with_callback(|gen, fitness, _| {
        pb.set_position(gen as u64);
        pb.set_message(format!("{:.4}", fitness));
    });

    pb.finish_with_message(format!("{:.4}", result.best_fitness));

    println!("\n最適化完了!");
    println!("最良フィットネス: {:.4}", result.best_fitness);
    println!("\n最良配列:");
    println!("{}", result.best_layout.format());

    // スコア詳細
    print_scores(&result.best_layout, &weights);

    // 保存
    save_layout(&result.best_layout, output);
}

/// TUIモードで実行
fn run_with_tui(corpus: &CorpusStats, config: GaConfig, weights: EvaluationWeights, output: &PathBuf) {
    use crate::tui::{run_tui_thread, TuiState};

    let state = Arc::new(Mutex::new(TuiState::new(config.generations)));
    let tui_state = Arc::clone(&state);
    
    // Ctrl+C ハンドラ設定（途中停止時も結果保存）
    let state_for_signal = Arc::clone(&state);
    let output_for_signal = output.clone();
    ctrlc::set_handler(move || {
        let s = state_for_signal.lock().unwrap();
        if let Some(ref best_layout) = s.best_layout {
            println!("\n\n中断されました。現在の最良結果を保存中...");
            save_layout(best_layout, &output_for_signal);
            println!("最良フィットネス: {:.4}", s.best_fitness);
            std::process::exit(0);
        } else {
            println!("\n\n中断されました（保存する結果がありません）");
            std::process::exit(1);
        }
    }).expect("Ctrl+Cハンドラ設定失敗");
    
    // TUIスレッド開始
    let tui_handle = run_tui_thread(tui_state);

    let mut ga = GeneticAlgorithm::with_weights(corpus.clone(), config.clone(), weights.clone());

    let result = ga.run_with_callback(|gen, fitness, layout| {
        let mut s = state.lock().unwrap();
        if !s.running {
            return;
        }
        s.update(gen, fitness, layout);
    });

    // TUI終了
    {
        let mut s = state.lock().unwrap();
        s.running = false;
    }
    let _ = tui_handle.join();

    println!("\n最適化完了!");
    println!("最良フィットネス: {:.4}", result.best_fitness);
    println!("\n最良配列:");
    println!("{}", result.best_layout.format());

    print_scores(&result.best_layout, &weights);
    save_layout(&result.best_layout, output);
}

/// マルチラン実行（プログレスバー）
fn run_multi(
    corpus: &CorpusStats,
    config: GaConfig,
    weights: EvaluationWeights,
    num_runs: usize,
    output: &PathBuf,
) {
    let actual_runs = num_runs.min(num_cpus::get());
    println!("マルチラン実行: {} 回（CPUコア数: {}）", actual_runs, num_cpus::get());

    let pb = ProgressBar::new(actual_runs as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} (Runs)")
            .unwrap()
            .progress_chars("#>-"),
    );

    let results = ga::run_multi(corpus.clone(), config, weights.clone(), actual_runs);
    pb.finish();

    print_multi_results(&results, &weights, output);
}

/// マルチラン実行（TUI付き）
fn run_multi_with_tui(
    corpus: &CorpusStats,
    config: GaConfig,
    weights: EvaluationWeights,
    num_runs: usize,
    output: &PathBuf,
) {
    use crate::tui::{run_tui_thread, TuiState};
    use std::sync::atomic::{AtomicUsize, Ordering};

    let actual_runs = num_runs.min(num_cpus::get());
    
    // 共有状態
    let state = Arc::new(Mutex::new(TuiState::new(config.generations)));
    let completed_runs = Arc::new(AtomicUsize::new(0));
    
    // TUIスレッド開始
    let tui_state = Arc::clone(&state);
    let tui_handle = run_tui_thread(tui_state);

    // 並列実行（各ランで最良をTUIに報告）
    let results: Vec<_> = (0..actual_runs)
        .into_iter()
        .map(|_| {
            let seed: u64 = rand::random();
            let mut run_config = config.clone();
            run_config.seed = seed;
            
            let state = Arc::clone(&state);
            let completed = Arc::clone(&completed_runs);
            
            let mut ga = ga::GeneticAlgorithm::with_weights(
                corpus.clone(),
                run_config.clone(),
                weights.clone(),
            );
            
            let result = ga.run_with_callback(|gen, fitness, layout| {
                let mut s = state.lock().unwrap();
                if !s.running {
                    return;
                }
                // 全ランで最良のものだけ更新
                if fitness > s.best_fitness {
                    s.update(gen, fitness, layout);
                }
            });
            
            completed.fetch_add(1, Ordering::SeqCst);
            result
        })
        .collect();

    // TUI終了
    {
        let mut s = state.lock().unwrap();
        s.running = false;
    }
    let _ = tui_handle.join();

    print_multi_results(&results, &weights, output);
}

/// マルチラン結果を表示
fn print_multi_results(results: &[ga::GaResult], weights: &EvaluationWeights, output: &PathBuf) {
    let (mean, stddev, min, max, best) = ga::summarize_results(results);

    println!("\n=== マルチラン結果 ===");
    println!("実行回数: {}", results.len());
    println!("平均フィットネス: {:.4}", mean);
    println!("標準偏差: {:.4}", stddev);
    println!("最小: {:.4}", min);
    println!("最大: {:.4}", max);

    println!("\n最良配列:");
    println!("{}", best.best_layout.format());

    print_scores(&best.best_layout, weights);
    save_layout(&best.best_layout, output);
}

/// スコア詳細を表示
fn print_scores(layout: &Layout, weights: &EvaluationWeights) {
    let s = &layout.scores;
    let w = weights;

    println!("\n=== スコア詳細 ===");
    println!("\nCore Metrics (乗算・指数):");
    println!("  同指連続低:     {:.2}% ^{:.2}", s.same_finger, w.same_finger);
    println!("  段飛ばし少:     {:.2}% ^{:.2}", s.row_skip, w.row_skip);
    println!("  ホームポジ率:   {:.2}% ^{:.2}", s.home_position, w.home_position);
    println!("  総打鍵コスト少: {:.2}% ^{:.2}", s.total_keystrokes, w.total_keystrokes);
    println!("  左右交互:       {:.2}% ^{:.2}", s.alternating, w.alternating);
    println!("  単打鍵率:       {:.2}% ^{:.2}", s.single_key, w.single_key);
    println!("  Colemak類似:    {:.2}% ^{:.2}", s.colemak_similarity, w.colemak_similarity);
    println!("  位置別コスト:   {:.2}% ^{:.2}", s.position_cost, w.position_cost);

    println!("\nBonus Metrics (加算):");
    println!("  リダイレクト少: {:.2} x {:.1}", s.redirect_low, w.redirect_low);
    println!("  月配列類似:     {:.2} x {:.1}", s.tsuki_similarity, w.tsuki_similarity);
    println!("  ロール率:       {:.2} x {:.1}", s.roll, w.roll);
    println!("  インロール:     {:.2} x {:.1}", s.inroll, w.inroll);
    println!("  アルペジオ:     {:.2} x {:.1}", s.arpeggio, w.arpeggio);
    println!("  覚えやすさ:     {:.2} x {:.1}", s.memorability, w.memorability);
    println!("  シフトバランス: {:.2} x {:.1}", s.shift_balance, w.shift_balance);
}

/// 配列をJSONファイルに保存
fn save_layout(layout: &Layout, path: &PathBuf) {
    let json = serde_json::json!({
        "name": "GA Optimized Layout",
        "fitness": layout.fitness,
        "scores": layout.scores,
        "layers": {
            "no_shift": layout.layers[0],
            "shift_a": layout.layers[1],
            "shift_b": layout.layers[2],
        }
    });

    match std::fs::write(path, serde_json::to_string_pretty(&json).unwrap()) {
        Ok(_) => println!("\n配列を保存: {:?}", path),
        Err(e) => eprintln!("\n保存エラー: {}", e),
    }
}
