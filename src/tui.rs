//! TUI (Terminal User Interface) モジュール
//!
//! ratatuiを使用したリアルタイム進捗表示

use std::io::{self, Stdout};
use std::sync::{Arc, Mutex};

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Chart, Dataset, Gauge, Paragraph, Wrap},
    Frame, Terminal,
};

use crate::evaluation::EvaluationWeights;
use crate::layout::Layout as KeyboardLayout;
use crate::corpus::CorpusStats;

/// TUI状態
pub struct TuiState {
    pub generation: usize,
    pub max_generations: usize,
    pub best_fitness: f64,
    pub fitness_history: Vec<f64>,
    pub best_layout: Option<KeyboardLayout>,
    pub weights: Option<EvaluationWeights>,
    pub corpus_stats: Option<Arc<CorpusStats>>,
    pub running: bool,
    pub debug: bool,
}

impl TuiState {
    pub fn new(max_generations: usize) -> Self {
        Self::new_with_debug(max_generations, false, None)
    }
    
    pub fn new_with_debug(max_generations: usize, debug: bool, corpus_stats: Option<Arc<CorpusStats>>) -> Self {
        Self {
            generation: 0,
            max_generations,
            best_fitness: 0.0,
            fitness_history: Vec::with_capacity(max_generations),
            best_layout: None,
            weights: None,
            corpus_stats,
            running: true,
            debug,
        }
    }
    
    pub fn set_weights(&mut self, weights: EvaluationWeights) {
        self.weights = Some(weights);
    }

    pub fn update(&mut self, generation: usize, fitness: f64, layout: &KeyboardLayout) {
        self.generation = generation;
        if fitness > self.best_fitness {
            self.best_fitness = fitness;
            self.best_layout = Some(layout.clone());
        }
        self.fitness_history.push(fitness);
    }
}

/// TUIアプリケーション
pub struct TuiApp {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TuiApp {
    /// TUIを初期化
    pub fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }

    /// TUIを描画
    pub fn draw(&mut self, state: &TuiState) -> io::Result<()> {
        self.terminal.draw(|f| {
            let main_chunks = if state.debug {
                // デバッグモード: 上部を圧縮、下部を拡大
                Layout::default()
                    .direction(Direction::Vertical)
                    .margin(1)
                    .constraints([
                        Constraint::Length(3),   // Progress bar
                        Constraint::Length(8),   // Graph (小さく)
                        Constraint::Min(30),     // Layout + Scores + Debug (大きく)
                    ])
                    .split(f.area())
            } else {
                // 通常モード
                Layout::default()
                    .direction(Direction::Vertical)
                    .margin(1)
                    .constraints([
                        Constraint::Length(3),   // Progress bar
                        Constraint::Length(12),  // Graph
                        Constraint::Percentage(50), // Layout + Scores
                    ])
                    .split(f.area())
            };

            let bottom_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(50), // Layout
                    Constraint::Percentage(50), // Scores + Weights
                ])
                .split(main_chunks[2]);

            render_progress(f, main_chunks[0], state);
            render_graph(f, main_chunks[1], state);
            
            if state.debug {
                // デバッグモード: 2段構成（上段：キーボードのみ、下段：デバッグ3カラム）
                let debug_rows = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(19),  // 上段: キーボードのみ（3行拡張）
                        Constraint::Min(20),     // 下段: デバッグ3カラム
                    ])
                    .split(main_chunks[2]);
                
                render_keyboard(f, debug_rows[0], state);
                render_debug_panel(f, debug_rows[1], state); // 下段でスコア+採点+位置コスト
            } else {
                // 通常モード: 2分割
                render_keyboard(f, bottom_chunks[0], state);
                render_scores_and_weights(f, bottom_chunks[1], state);
            }
        })?;
        Ok(())
    }

    /// イベントをポーリング（ノンブロッキング）
    pub fn poll_event(&self) -> io::Result<bool> {
        if event::poll(std::time::Duration::from_millis(10))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') || key.code == KeyCode::Esc {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// TUIを終了
    pub fn cleanup(mut self) -> io::Result<()> {
        disable_raw_mode()?;
        execute!(self.terminal.backend_mut(), LeaveAlternateScreen)?;
        self.terminal.show_cursor()?;
        Ok(())
    }
}

/// プログレスバーを描画（ETA追加）
fn render_progress(f: &mut Frame, area: Rect, state: &TuiState) {
    let progress = if state.max_generations > 0 {
        state.generation as f64 / state.max_generations as f64
    } else {
        0.0
    };

    // ETA推定（簡易版：最後10世代の平均速度から計算）
    let eta_str = if state.generation > 10 && state.generation < state.max_generations {
        let remaining = state.max_generations - state.generation;
        let eta_seconds = remaining as u64; // 1世代≈1秒と仮定（簡易版）
        let minutes = eta_seconds / 60;
        let seconds = eta_seconds % 60;
        format!(" | ETA: {}m{}s", minutes, seconds)
    } else {
        String::new()
    };

    let debug_indicator = if state.debug { " [★DEBUG★]" } else { "" };
    let title = format!("Progress{}", debug_indicator);
    
    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title(title))
        .gauge_style(Style::default().fg(Color::Cyan).bg(Color::Black))
        .percent((progress * 100.0) as u16)
        .label(format!(
            "Gen {}/{} | Best: {:.4}{}",
            state.generation, state.max_generations, state.best_fitness, eta_str
        ));

    f.render_widget(gauge, area);
}

/// フィットネスグラフを描画
fn render_graph(f: &mut Frame, area: Rect, state: &TuiState) {
    let data: Vec<(f64, f64)> = state
        .fitness_history
        .iter()
        .enumerate()
        .map(|(i, &f)| (i as f64, f))
        .collect();

    if data.is_empty() {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Fitness History");
        f.render_widget(block, area);
        return;
    }

    let min_fitness = state
        .fitness_history
        .iter()
        .cloned()
        .fold(f64::INFINITY, f64::min)
        .max(0.0);
    let max_fitness = state
        .fitness_history
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max)
        .max(min_fitness + 1.0);

    let datasets = vec![Dataset::default()
        .name("Fitness")
        .marker(symbols::Marker::Braille)
        .style(Style::default().fg(Color::Yellow))
        .data(&data)];

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Fitness History (Gen 0 to Max)"),
        )
        .x_axis(
            Axis::default()
                .title("Generation")
                .style(Style::default().fg(Color::Gray))
                .bounds([0.0, state.max_generations as f64])
                .labels(vec![
                    Span::raw("0"),
                    Span::raw(format!("{}", state.max_generations / 2)),
                    Span::raw(format!("{}", state.max_generations)),
                ]),
        )
        .y_axis(
            Axis::default()
                .title("Fitness")
                .style(Style::default().fg(Color::Gray))
                .bounds([min_fitness - 5.0, max_fitness + 5.0])
                .labels(vec![
                    Span::raw(format!("{:.0}", min_fitness)),
                    Span::raw(format!("{:.0}", (min_fitness + max_fitness) / 2.0)),
                    Span::raw(format!("{:.0}", max_fitness)),
                ]),
        );

    f.render_widget(chart, area);
}

/// キーボード配列を描画
fn render_keyboard(f: &mut Frame, area: Rect, state: &TuiState) {
    let layout = match &state.best_layout {
        Some(l) => l,
        None => {
            let block = Block::default()
                .borders(Borders::ALL)
                .title("Best Layout");
            f.render_widget(block, area);
            return;
        }
    };

    let mut lines: Vec<Line> = vec![Line::from(Span::styled(
        format!("Fitness: {:.4}", state.best_fitness),
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
    ))];

    lines.push(Line::from(""));

    for (layer_idx, layer_name) in ["Layer 0 (無シフト)", "Layer 1 (☆)", "Layer 2 (★)"]
        .iter()
        .enumerate()
    {
        lines.push(Line::from(Span::styled(
            format!("{}:", layer_name),
            Style::default().fg(Color::Cyan),
        )));

        for row in 0..3 {
            let row_str: String = layout.layers[layer_idx][row]
                .iter()
                .map(|&c| if c == '　' { '□' } else { c })
                .collect::<Vec<_>>()
                .iter()
                .map(|c| format!("{} ", c))
                .collect();
            lines.push(Line::from(format!("  {}", row_str)));
        }
        lines.push(Line::from(""));
    }

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Current Best Layout"),
    );

    f.render_widget(paragraph, area);
}

/// Colemak一致詳細を計算（評価関数と同じロジック）
fn calc_colemak_match_detail(layers: &[[[char; 10]; 3]; 3]) -> (usize, usize, usize) {
    use crate::layout::{romaji_phonemes, COLEMAK_POSITIONS};
    use std::collections::HashMap;
    
    // COLEMAK_POSITIONSから音素→位置のマップを作成
    let mut phoneme_pos: HashMap<&str, (usize, usize)> = HashMap::new();
    for &(phoneme, row, col) in COLEMAK_POSITIONS {
        phoneme_pos.insert(phoneme, (row, col));
    }
    
    let mut perfect = 0;  // 完全一致（両音素とも位置一致）
    let mut partial = 0;  // 部分一致（片方のみ一致または行/手一致）
    let mut total = 0;
    
    // 評価関数と同じく全レイヤーを評価（Layer 0重視）
    for layer in 0..3 {
        let layer_weight = if layer == 0 { 1.0 } else { 0.3 };
        
        for row in 0..3 {
            for col in 0..10 {
                let c = layers[layer][row][col];
                
                // 除外文字
                if c == '☆' || c == '★' || c == '、' || c == '。' || c == '　' ||
                   c == 'ー' || c == 'っ' || c == 'ゃ' || c == 'ゅ' || c == 'ょ' ||
                   c == 'ぁ' || c == 'ぃ' || c == 'ぅ' || c == 'ぇ' || c == 'ぉ' ||
                   c == '゛' || c == '゜' {
                    continue;
                }
                
                let (consonant, vowel) = romaji_phonemes(c);
                
                // 音素がない文字はスキップ
                if consonant.is_none() && vowel.is_none() {
                    continue;
                }
                
                total += 1;
                
                let mut cons_score = 0.0;
                let mut vowel_score = 0.0;
                
                // 子音チェック
                if let Some(cons) = consonant {
                    if let Some(&(exp_row, exp_col)) = phoneme_pos.get(cons) {
                        if row == exp_row && col == exp_col {
                            cons_score = 1.0;  // 完全一致
                        } else if row == exp_row {
                            cons_score = 0.5;  // 行一致
                        } else if (col < 5 && exp_col < 5) || (col >= 5 && exp_col >= 5) {
                            cons_score = 0.25; // 手一致
                        }
                    }
                }
                
                // 母音チェック
                if let Some(vow) = vowel {
                    if let Some(&(exp_row, exp_col)) = phoneme_pos.get(vow) {
                        if row == exp_row && col == exp_col {
                            vowel_score = 1.0;  // 完全一致
                        } else if row == exp_row {
                            vowel_score = 0.5;  // 行一致
                        } else if (col < 5 && exp_col < 5) || (col >= 5 && exp_col >= 5) {
                            vowel_score = 0.25; // 手一致
                        }
                    }
                }
                
                // スコアリング
                // 音素の種類で判定を分岐
                let is_vowel_only = consonant.is_none() && vowel.is_some();
                let is_consonant_only = consonant.is_some() && vowel.is_none();
                
                if is_vowel_only {
                    // 母音のみ（あいうえお）：完全一致なら◎、行一致以上で○
                    if vowel_score >= 1.0 {
                        perfect += 1;
                    } else if vowel_score >= 0.5 {
                        partial += 1;
                    }
                } else if is_consonant_only {
                    // 子音のみ（ん）：完全一致なら◎、行一致以上で○
                    if cons_score >= 1.0 {
                        perfect += 1;
                    } else if cons_score >= 0.5 {
                        partial += 1;
                    }
                } else {
                    // 子音+母音：両方完全一致で◎、どちらか完全一致で○
                    let has_perfect_match = cons_score >= 1.0 || vowel_score >= 1.0;
                    let total_score = cons_score + vowel_score;
                    
                    if total_score >= 1.8 {
                        perfect += 1;
                    } else if has_perfect_match {
                        partial += 1;
                    }
                }
            }
        }
    }
    
    (perfect, partial, total)
}

/// 月配列一致詳細を計算（ヘルパー関数）
/// 戻り値: (L0一致, L0総数, L1一致, L1総数, L2一致, L2総数)
fn calc_tsuki_match_detail(layers: &[[[char; 10]; 3]; 3]) -> (usize, usize, usize, usize, usize, usize) {
    // 月配列: Layer 0 = 表面, Layer 1 = 裏面
    let tsuki_layers = [
        [
            ['そ', 'こ', 'し', 'て', 'ょ', 'つ', 'ん', 'い', 'の', 'り'],
            ['は', 'か', '☆', 'と', 'た', 'く', 'う', '★', '゛', 'き'],
            ['す', 'け', 'に', 'な', 'さ', 'っ', 'る', '、', '。', '゜'],
        ],
        [
            ['ぁ', 'ひ', 'ほ', 'ふ', 'め', 'ぬ', 'え', 'み', 'や', 'ぇ'],
            ['ぃ', 'を', 'ら', 'あ', 'よ', 'ま', 'お', 'も', 'わ', 'ゆ'],
            ['ぅ', 'へ', 'せ', 'ゅ', 'ゃ', 'む', 'ろ', 'ね', 'ー', 'ぉ'],
        ],
    ];

    let mut l0_matched = 0;
    let mut l0_total = 0;
    let mut l1_matched = 0;
    let mut l1_total = 0;
    let mut l2_matched = 0;
    let mut l2_total = 0;

    for ga_layer in 0..3 {
        // GA Layer 0 → 月 Layer 0（表面）
        // GA Layer 1, 2 → 月 Layer 1（裏面）
        let tsuki_layer = if ga_layer == 0 { 0 } else { 1 };

        for row in 0..3 {
            for col in 0..10 {
                let kana = layers[ga_layer][row][col];
                let tsuki_char = tsuki_layers[tsuki_layer][row][col];

                if kana == '★' || kana == '☆' || kana == '、' || kana == '。' || kana == '　' ||
                   tsuki_char == '★' || tsuki_char == '☆' || tsuki_char == '、' || tsuki_char == '。' ||
                   tsuki_char == '゛' || tsuki_char == '゜' || tsuki_char == '　' {
                    continue;
                }

                let is_match = kana == tsuki_char;
                match ga_layer {
                    0 => {
                        l0_total += 1;
                        if is_match { l0_matched += 1; }
                    }
                    1 => {
                        l1_total += 1;
                        if is_match { l1_matched += 1; }
                    }
                    2 => {
                        l2_total += 1;
                        if is_match { l2_matched += 1; }
                    }
                    _ => {}
                }
            }
        }
    }
    (l0_matched, l0_total, l1_matched, l1_total, l2_matched, l2_total)
}

/// キーごとの位置コストを計算（評価ロジックと同じ）
fn calc_position_cost_for_key(pos: &crate::layout::KeyPos) -> f64 {
    // base_cost計算（評価と同じロジック）
    let base_cost = match pos.row {
        1 => 1.0,  // ホーム
        0 => match pos.col {
            0 | 9 => 3.0,     // 上段外側
            1 | 8 => 2.5,     // 上段薬指
            _ => 2.0,         // 上段その他
        },
        2 => match pos.col {
            0 | 9 => 3.0,     // 下段外側
            _ => 2.5,         // 下段その他
        },
        _ => 4.0,
    };
    
    let mut multiplier = 1.0;
    let layer_penalty = if pos.layer == 0 { 1.0 } else { 1.05 };
    
    if pos.layer == 1 {  // ☆シフト
        multiplier = 3.0 * layer_penalty;
        if pos.col == 7 && pos.row != 1 {  // Ver: ☆の上下
            multiplier += 27.0;
        }
        if pos.col >= 8 {  // Out: ☆より小指側
            multiplier += 9.0;
        }
    } else if pos.layer == 2 {  // ★シフト
        multiplier = 3.0 * layer_penalty;
        if pos.col == 2 && pos.row != 1 {  // Ver: ★の上下
            multiplier += 27.0;
        }
        if pos.col <= 1 {  // Out: ★より小指側
            multiplier += 9.0;
        }
    }
    
    base_cost * multiplier
}

/// デバッグパネルを描画（全計算過程・3カラム）
fn render_debug_panel(f: &mut Frame, area: Rect, state: &TuiState) {
    // 3カラムに分割
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30), // 左：Core/Bonus計算式
            Constraint::Percentage(35), // 中央：キーごと採点
            Constraint::Percentage(35), // 右：位置コスト
        ])
        .split(area);
    
    let mut left_lines = vec![];
    let mut center_lines = vec![];
    let mut right_lines = vec![];
    
    if let (Some(layout), Some(w)) = (&state.best_layout, &state.weights) {
        let s = &layout.scores;
        let layers = &layout.layers;
        
        // ========== 左カラム: Similarity & Scores + 計算式 ==========
        left_lines.push(Line::from(Span::styled(
            "=== Similarity ===",
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        )));
        
        // Similarity詳細
        let (colemak_perfect, colemak_partial, colemak_total) = calc_colemak_match_detail(&layers);
        left_lines.push(Line::from(format!(
            "Colemak: {:.1}% (◎{} ○{} ×{})",
            s.colemak_similarity, colemak_perfect, colemak_partial,
            colemak_total - colemak_perfect - colemak_partial
        )));
        
        let (l0_m, l0_t, l1_m, l1_t, l2_m, l2_t) = calc_tsuki_match_detail(&layers);
        let total_match = l0_m + l1_m + l2_m;
        let total_all = l0_t + l1_t + l2_t;
        left_lines.push(Line::from(format!(
            "月配列: {:.1}% (○{}/{})",
            s.tsuki_similarity, total_match, total_all
        )));
        left_lines.push(Line::from(format!(
            "  L0:{}/{} L1:{}/{} L2:{}/{}",
            l0_m, l0_t, l1_m, l1_t, l2_m, l2_t
        )));
        
        left_lines.push(Line::from(""));
        left_lines.push(Line::from(Span::styled(
            "=== Core Metrics ===",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )));

        // Core計算式（基本5指標のみ）
        left_lines.push(Line::from(format!("同指連続低: {:.1}%^{:.1}={:.3}",
            s.same_finger, w.same_finger, (s.same_finger/100.0).powf(w.same_finger))));
        left_lines.push(Line::from(format!("段越え低: {:.1}%^{:.1}={:.3}",
            s.row_skip, w.row_skip, (s.row_skip/100.0).powf(w.row_skip))));
        left_lines.push(Line::from(format!("ホーム率: {:.1}%^{:.1}={:.3}",
            s.home_position, w.home_position, (s.home_position/100.0).powf(w.home_position))));
        left_lines.push(Line::from(format!("打鍵少: {:.1}%^{:.1}={:.3}",
            s.total_keystrokes, w.total_keystrokes, (s.total_keystrokes/100.0).powf(w.total_keystrokes))));
        left_lines.push(Line::from(format!("左右交互: {:.1}%^{:.1}={:.3}",
            s.alternating, w.alternating, (s.alternating/100.0).powf(w.alternating))));

        let core_product = (s.same_finger/100.0).powf(w.same_finger)
            * (s.row_skip/100.0).powf(w.row_skip)
            * (s.home_position/100.0).powf(w.home_position)
            * (s.total_keystrokes/100.0).powf(w.total_keystrokes)
            * (s.alternating/100.0).powf(w.alternating);
        let total_weight = w.same_finger + w.row_skip + w.home_position
            + w.total_keystrokes + w.alternating;
        let core_mult = core_product.powf(1.0 / total_weight) * 100.0;
        
        left_lines.push(Line::from(""));
        left_lines.push(Line::from(format!("→Core総合: {:.4}", core_mult)));
        
        // Bonus計算式
        left_lines.push(Line::from(""));
        left_lines.push(Line::from(Span::styled(
            "=== Bonus ===",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )));
        left_lines.push(Line::from(format!("単打率: {:.1}×{:.1}={:.1}",
            s.single_key, w.single_key, s.single_key * w.single_key)));
        left_lines.push(Line::from(format!("Colemak: {:.1}×{:.1}={:.1}",
            s.colemak_similarity, w.colemak_similarity, s.colemak_similarity * w.colemak_similarity)));
        left_lines.push(Line::from(format!("位置コスト: {:.1}×{:.1}={:.1}",
            s.position_cost, w.position_cost, s.position_cost * w.position_cost)));
        left_lines.push(Line::from(format!("リダイレクト低: {:.1}×{:.1}={:.1}",
            s.redirect_low, w.redirect_low, s.redirect_low * w.redirect_low)));
        left_lines.push(Line::from(format!("月類似: {:.1}×{:.1}={:.1}",
            s.tsuki_similarity, w.tsuki_similarity, s.tsuki_similarity * w.tsuki_similarity)));
        left_lines.push(Line::from(format!("ロール: {:.1}×{:.1}={:.1}",
            s.roll, w.roll, s.roll * w.roll)));
        left_lines.push(Line::from(format!("インロール: {:.1}×{:.1}={:.1}",
            s.inroll, w.inroll, s.inroll * w.inroll)));
        left_lines.push(Line::from(format!("アルペジオ: {:.1}×{:.1}={:.1}",
            s.arpeggio, w.arpeggio, s.arpeggio * w.arpeggio)));
        left_lines.push(Line::from(format!("覚えやすさ: {:.1}×{:.1}={:.1}",
            s.memorability, w.memorability, s.memorability * w.memorability)));
        left_lines.push(Line::from(format!("シフトバランス: {:.1}×{:.1}={:.1}",
            s.shift_balance, w.shift_balance, s.shift_balance * w.shift_balance)));

        let additive_bonus = s.single_key * w.single_key
            + s.colemak_similarity * w.colemak_similarity
            + s.position_cost * w.position_cost
            + s.redirect_low * w.redirect_low
            + s.tsuki_similarity * w.tsuki_similarity
            + s.roll * w.roll
            + s.inroll * w.inroll
            + s.arpeggio * w.arpeggio
            + s.memorability * w.memorability
            + s.shift_balance * w.shift_balance;
        let bonus_scale = (w.single_key + w.colemak_similarity + w.position_cost
            + w.redirect_low + w.tsuki_similarity + w.roll
            + w.inroll + w.arpeggio + w.memorability + w.shift_balance) * 100.0;
        
        left_lines.push(Line::from(""));
        left_lines.push(Line::from(format!("→Bonus: {:.2}/{:.0}={:.4}",
            additive_bonus, bonus_scale, additive_bonus/bonus_scale)));
        
        // 最終Fitness
        let final_fitness = core_mult * (1.0 + additive_bonus / bonus_scale);
        left_lines.push(Line::from(""));
        left_lines.push(Line::from(Span::styled(
            format!("■ Final: {:.4}", final_fitness),
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        )));
        left_lines.push(Line::from(format!("{:.2}×(1+{:.4})={:.4}",
            core_mult, additive_bonus/bonus_scale, final_fitness)));
        
        // ========== 右カラム ==========
        right_lines.push(Line::from(Span::styled(
            "=== 位置コスト ===",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )));
        right_lines.push(Line::from(""));
        
        // 位置コスト詳細（2段表示：文字[頻度] / (コスト)）
        use crate::layout::KeyPos;
        
        // 頻度マップを取得
        let freq_map: std::collections::HashMap<char, f64> = if let Some(ref corpus) = state.corpus_stats {
            corpus.char_freq.iter()
                .map(|(ch, &freq)| (*ch, freq as f64))
                .collect()
        } else {
            std::collections::HashMap::new()
        };
        
        for layer in 0..3 {
            let layer_name = match layer {
                0 => "L0", 1 => "L1", 2 => "L2", _ => "",
            };
            right_lines.push(Line::from(format!("{}:", layer_name)));
            
            for row in 0..3 {
                let mut row_line = String::new();
                
                for col in 0..10 {
                    let kana = layers[layer][row][col];
                    let pos = KeyPos::new(layer, row, col);
                    let cost = calc_position_cost_for_key(&pos);
                    
                    // 頻度取得（1-gramから）
                    let freq = freq_map.get(&kana).copied().unwrap_or(0.0);
                    
                    // 頻度を短く表示（k単位）
                    let freq_str = if freq >= 10000.0 {
                        format!("{}k", (freq / 1000.0) as i32)
                    } else if freq >= 1000.0 {
                        format!("{:.1}k", freq / 1000.0)
                    } else if freq > 0.0 {
                        format!("{}", freq as i32)
                    } else {
                        "0".to_string()
                    };
                    
                    // コストをそのまま表示（1-99の範囲）
                    let display_cost = (cost as i32).min(99);
                    
                    // 文字:頻度(コスト) 形式
                    row_line.push_str(&format!("{}:{}({:>2}) ", kana, freq_str, display_cost));
                }
                
                right_lines.push(Line::from(format!(" {}", row_line)));
            }
            right_lines.push(Line::from(""));
        }
        
        right_lines.push(Line::from("L0: 上段外側・下段外側"));
        right_lines.push(Line::from("L1: ☆(col7)上下Ver+27,Out+9"));
        right_lines.push(Line::from("L2: ★(col2)上下Ver+27,Out+9"));
        
        // ========== 中央カラム: キーごと採点 ==========
        center_lines.push(Line::from(Span::styled(
            "=== キーごと採点 ===",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )));
        center_lines.push(Line::from(""));
        
        // Colemak採点（全3レイヤー）
        center_lines.push(Line::from(Span::styled(
            "■ Colemak (L0/L1/L2)",
            Style::default().fg(Color::Yellow),
        )));
        
        use crate::layout::{romaji_phonemes, COLEMAK_POSITIONS};
        use std::collections::HashMap;
        
        let mut phoneme_pos: HashMap<&str, (usize, usize)> = HashMap::new();
        for &(phoneme, row, col) in COLEMAK_POSITIONS {
            phoneme_pos.insert(phoneme, (row, col));
        }
        
        // 全3レイヤーを表示
        for layer in 0..3 {
            let layer_name = match layer {
                0 => "L0:", 1 => "L1:", 2 => "L2:", _ => "",
            };
            center_lines.push(Line::from(layer_name));
            
            for row in 0..3 {
                let mut match_line = String::new();
                for col in 0..10 {
                    let c = layers[layer][row][col];
                    
                    if c == '☆' || c == '★' || c == '、' || c == '。' || c == '　' ||
                       c == 'ー' || c == 'っ' || c == 'ゃ' || c == 'ゅ' || c == 'ょ' ||
                       c == 'ぁ' || c == 'ぃ' || c == 'ぅ' || c == 'ぇ' || c == 'ぉ' ||
                       c == '゛' || c == '゜' {
                        match_line.push_str("  ");
                        continue;
                    }
                    
                    let (consonant, vowel) = romaji_phonemes(c);
                    if consonant.is_none() && vowel.is_none() {
                        match_line.push_str("  ");
                        continue;
                    }
                    
                    let mut cons_score = 0.0;
                    let mut vowel_score = 0.0;
                    
                    if let Some(cons) = consonant {
                        if let Some(&(exp_row, exp_col)) = phoneme_pos.get(cons) {
                            if row == exp_row && col == exp_col {
                                cons_score = 1.0;
                            } else if row == exp_row {
                                cons_score = 0.5;
                            } else if (col < 5 && exp_col < 5) || (col >= 5 && exp_col >= 5) {
                                cons_score = 0.25;
                            }
                        }
                    }
                    
                    if let Some(vow) = vowel {
                        if let Some(&(exp_row, exp_col)) = phoneme_pos.get(vow) {
                            if row == exp_row && col == exp_col {
                                vowel_score = 1.0;
                            } else if row == exp_row {
                                vowel_score = 0.5;
                            } else if (col < 5 && exp_col < 5) || (col >= 5 && exp_col >= 5) {
                                vowel_score = 0.25;
                            }
                        }
                    }
                    
                    // 音素の種類で判定を分岐
                    let is_vowel_only = consonant.is_none() && vowel.is_some();
                    let is_consonant_only = consonant.is_some() && vowel.is_none();
                    
                    if is_vowel_only {
                        // 母音のみ（あいうえお）：完全一致なら◎、行一致以上で○
                        if vowel_score >= 1.0 {
                            match_line.push('◎');
                        } else if vowel_score >= 0.5 {
                            match_line.push('○');
                        } else {
                            match_line.push('×');
                        }
                    } else if is_consonant_only {
                        // 子音のみ（ん）：完全一致なら◎、行一致以上で○
                        if cons_score >= 1.0 {
                            match_line.push('◎');
                        } else if cons_score >= 0.5 {
                            match_line.push('○');
                        } else {
                            match_line.push('×');
                        }
                    } else {
                        // 子音+母音：両方完全一致で◎、どちらか完全一致で○
                        let has_perfect_match = cons_score >= 1.0 || vowel_score >= 1.0;
                        let total_score = cons_score + vowel_score;
                        
                        if total_score >= 1.8 {
                            match_line.push('◎');
                        } else if has_perfect_match {
                            match_line.push('○');
                        } else {
                            match_line.push('×');
                        }
                    }
                    match_line.push(' ');
                }
                center_lines.push(Line::from(format!(" {}", match_line)));
            }
            center_lines.push(Line::from(""));
        }
        
        // 月配列採点
        center_lines.push(Line::from(""));
        center_lines.push(Line::from(Span::styled(
            "■ 月配列 (L0+L1)",
            Style::default().fg(Color::Yellow),
        )));
        
        let tsuki_layers = [
            [
                ['そ', 'こ', 'し', 'て', 'ょ', 'つ', 'ん', 'い', 'の', 'り'],
                ['は', 'か', '☆', 'と', 'た', 'く', 'う', '★', '゛', 'き'],
                ['す', 'け', 'に', 'な', 'さ', 'っ', 'る', '、', '。', '゜'],
            ],
            [
                ['ぁ', 'ひ', 'ほ', 'ふ', 'め', 'ぬ', 'え', 'み', 'や', 'ぇ'],
                ['ぃ', 'を', 'ら', 'あ', 'よ', 'ま', 'お', 'も', 'わ', 'ゆ'],
                ['ぅ', 'へ', 'せ', 'ゅ', 'ゃ', 'む', 'ろ', 'ね', 'ー', 'ぉ'],
            ],
        ];
        
        for layer in 0..2 {
            let layer_name = match layer {
                0 => "L0", 1 => "L1", _ => "",
            };
            center_lines.push(Line::from(format!("{}:", layer_name)));
            
            for row in 0..3 {
                let mut match_line = String::new();
                for col in 0..10 {
                    let kana = layers[layer][row][col];
                    let tsuki_char = tsuki_layers[layer][row][col];
                    
                    if kana == '★' || kana == '☆' || kana == '、' || kana == '。' || kana == '　' ||
                       tsuki_char == '★' || tsuki_char == '☆' || tsuki_char == '、' || tsuki_char == '。' ||
                       tsuki_char == '゛' || tsuki_char == '゜' || tsuki_char == '　' {
                        match_line.push_str("  ");
                        continue;
                    }
                    
                    if kana == tsuki_char {
                        match_line.push('○');
                    } else {
                        match_line.push('×');
                    }
                    match_line.push(' ');
                }
                center_lines.push(Line::from(format!(" {}", match_line)));
            }
        }
        
    } else {
        left_lines.push(Line::from("計算中..."));
        center_lines.push(Line::from("計算中..."));
        right_lines.push(Line::from("計算中..."));
    }
    
    // 左カラム描画
    let left_para = Paragraph::new(left_lines)
        .block(Block::default()
            .title("L: Scores+計算式")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green)))
        .wrap(Wrap { trim: false });
    f.render_widget(left_para, columns[0]);
    
    // 中央カラム描画
    let center_para = Paragraph::new(center_lines)
        .block(Block::default()
            .title("C: キーごと採点")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)))
        .wrap(Wrap { trim: false });
    f.render_widget(center_para, columns[1]);
    
    // 右カラム描画
    let right_para = Paragraph::new(right_lines)
        .block(Block::default()
            .title("R: 位置コスト")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)))
        .wrap(Wrap { trim: false });
    f.render_widget(right_para, columns[2]);
}
fn render_scores_and_weights(f: &mut Frame, area: Rect, state: &TuiState) {
    let layout = match &state.best_layout {
        Some(l) => l,
        None => {
            let block = Block::default()
                .borders(Borders::ALL)
                .title("Scores & Weights");
            f.render_widget(block, area);
            return;
        }
    };

    let mut lines: Vec<Line> = vec![];
    let s = &layout.scores;
    let w = state.weights.as_ref();

    // Similarity & Scores（計算式付き）
    lines.push(Line::from(Span::styled(
        "=== Similarity & Scores ===",
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
    )));
    // Colemak一致率の詳細計算
    let (colemak_perfect, colemak_partial, colemak_total) = calc_colemak_match_detail(&layout.layers);
    lines.push(Line::from(format!(
        "Colemak:    {:.1}% (◎{} ○{} ×{})",
        s.colemak_similarity, colemak_perfect, colemak_partial,
        colemak_total - colemak_perfect - colemak_partial
    )));
    
    // 月配列一致率の詳細計算
    let (l0_m, l0_t, l1_m, l1_t, l2_m, l2_t) = calc_tsuki_match_detail(&layout.layers);
    let tsuki_match = l0_m + l1_m + l2_m;
    let tsuki_total = l0_t + l1_t + l2_t;
    lines.push(Line::from(format!(
        "月配列:     {:.1}% (○{} ×{})",
        s.tsuki_similarity, tsuki_match, tsuki_total - tsuki_match
    )));
    // 位置コストはCore metricsに移動

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "=== Core Metrics (乗算) ===",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )));

    if let Some(weights) = w {
        lines.push(Line::from(format!(
            "同指連続低: {:.1}% ^{:.1}",
            s.same_finger, weights.same_finger
        )));
        lines.push(Line::from(format!(
            "段越え低:   {:.1}% ^{:.1}",
            s.row_skip, weights.row_skip
        )));
        lines.push(Line::from(format!(
            "ホーム率:   {:.1}% ^{:.1}",
            s.home_position, weights.home_position
        )));
        lines.push(Line::from(format!(
            "打鍵少:     {:.1}% ^{:.1}",
            s.total_keystrokes, weights.total_keystrokes
        )));
        lines.push(Line::from(format!(
            "左右交互:   {:.1}% ^{:.1}",
            s.alternating, weights.alternating
        )));

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "=== Bonus (加算) ===",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));

        lines.push(Line::from(format!(
            "単打率:     {:.1}% ×{:.1}",
            s.single_key, weights.single_key
        )));
        lines.push(Line::from(format!(
            "位置コスト: {:.1}% ×{:.1}",
            s.position_cost, weights.position_cost
        )));
        lines.push(Line::from(format!(
            "ロール:     {:.1}% ×{:.1}",
            s.roll, weights.roll
        )));
        lines.push(Line::from(format!(
            "インロール: {:.1}% ×{:.1}",
            s.inroll, weights.inroll
        )));
        lines.push(Line::from(format!(
            "アルペジオ: {:.1}% ×{:.1}",
            s.arpeggio, weights.arpeggio
        )));
        lines.push(Line::from(format!(
            "リダイレクト低: {:.1}% ×{:.1}",
            s.redirect_low, weights.redirect_low
        )));
    } else {
        lines.push(Line::from("重み情報なし"));
    }

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Similarity & Scores"),
    );

    f.render_widget(paragraph, area);
}

/// TUIスレッドを実行
pub fn run_tui_thread(state: Arc<Mutex<TuiState>>) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut app = match TuiApp::new() {
            Ok(app) => app,
            Err(e) => {
                eprintln!("TUI error: {}", e);
                return;
            }
        };

        loop {
            {
                let state = state.lock().unwrap();
                if !state.running {
                    break;
                }
                if let Err(e) = app.draw(&state) {
                    eprintln!("TUI draw error: {}", e);
                    break;
                }
            }

            match app.poll_event() {
                Ok(true) => {
                    let mut state = state.lock().unwrap();
                    state.running = false;
                    break;
                }
                Err(e) => {
                    eprintln!("TUI event error: {}", e);
                    break;
                }
                _ => {}
            }

            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        if let Err(e) = app.cleanup() {
            eprintln!("TUI cleanup error: {}", e);
        }
    })
}
