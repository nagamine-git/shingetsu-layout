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
    /// マルチラン用: 各ランの状態 (run_id, fitness, layout)
    pub multi_run_states: Vec<(usize, f64, Option<KeyboardLayout>)>,
    /// マルチランモードかどうか
    pub multi_run_mode: bool,
    /// 完了したラン数
    pub completed_runs: usize,
    /// 総ラン数
    pub total_runs: usize,
    /// Initial配列（プレビュー用）
    pub initial_layout: KeyboardLayout,
    /// 2番目に良い配列
    pub second_best_layout: Option<KeyboardLayout>,
    pub second_best_fitness: f64,
    /// 表示モード: 0=best, 1=2nd_best, 2=initial
    pub view_mode: usize,
    /// デバッグパネル表示フラグ: [1:Scores, 2:KeyScore, 3:PositionCost]
    pub debug_panel_visible: [bool; 3],
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
            multi_run_states: Vec::new(),
            multi_run_mode: false,
            completed_runs: 0,
            total_runs: 0,
            initial_layout: KeyboardLayout::improved_custom(),
            second_best_layout: None,
            second_best_fitness: 0.0,
            view_mode: 0,
            debug_panel_visible: [true, true, true], // 全パネル初期表示
        }
    }

    /// マルチランモードを有効化
    pub fn enable_multi_run(&mut self, total_runs: usize) {
        self.multi_run_mode = true;
        self.total_runs = total_runs;
        self.multi_run_states = (0..total_runs.min(4))
            .map(|i| (i, 0.0, None))
            .collect();
    }

    /// マルチランの状態を更新
    pub fn update_multi_run(&mut self, run_id: usize, fitness: f64, layout: &KeyboardLayout) {
        if run_id < self.multi_run_states.len() {
            let current = &mut self.multi_run_states[run_id];
            if fitness > current.1 {
                current.1 = fitness;
                current.2 = Some(layout.clone());
            }
        }
        // 全体のベストも更新
        if fitness > self.best_fitness {
            self.best_fitness = fitness;
            self.best_layout = Some(layout.clone());
        }
    }
    
    pub fn set_weights(&mut self, weights: EvaluationWeights) {
        self.weights = Some(weights);
    }

    pub fn update(&mut self, generation: usize, fitness: f64, layout: &KeyboardLayout, second_best: Option<(f64, &KeyboardLayout)>) {
        self.generation = generation;
        if fitness > self.best_fitness {
            self.best_fitness = fitness;
            self.best_layout = Some(layout.clone());
        }
        // 2nd bestも更新
        if let Some((second_fitness, second_layout)) = second_best {
            if second_fitness > self.second_best_fitness {
                self.second_best_fitness = second_fitness;
                self.second_best_layout = Some(second_layout.clone());
            }
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
            if state.debug {
                // デバッグモード: 2カラムレイアウト
                let main_columns = Layout::default()
                    .direction(Direction::Horizontal)
                    .margin(1)
                    .constraints([
                        Constraint::Percentage(25), // 左: Progress + Graph
                        Constraint::Percentage(75), // 右: Layout + Debug panel
                    ])
                    .split(f.area());

                // 左カラム: Progress + Graph (縦に配置)
                let left_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),   // Progress bar
                        Constraint::Min(10),     // Graph
                    ])
                    .split(main_columns[0]);

                render_progress(f, left_chunks[0], state);
                render_graph(f, left_chunks[1], state);

                // 右カラム: キーボード + デバッグパネル
                let right_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(25),  // キーボード (Layer 0-3全表示に拡大)
                        Constraint::Min(20),     // デバッグパネル
                    ])
                    .split(main_columns[1]);

                render_keyboard(f, right_chunks[0], state);
                render_debug_panel(f, right_chunks[1], state);
            } else {
                // 通常モード
                let main_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(1)
                    .constraints([
                        Constraint::Length(3),   // Progress bar
                        Constraint::Length(12),  // Graph
                        Constraint::Percentage(50), // Layout + Scores
                    ])
                    .split(f.area());

                let bottom_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(50), // Layout
                        Constraint::Percentage(50), // Scores + Weights
                    ])
                    .split(main_chunks[2]);

                render_progress(f, main_chunks[0], state);
                render_graph(f, main_chunks[1], state);
                render_keyboard(f, bottom_chunks[0], state);
                render_scores_and_weights(f, bottom_chunks[1], state);
            }
        })?;
        Ok(())
    }

    /// イベントをポーリング（ノンブロッキング）
    /// 戻り値: Some(char)=キー操作, None=継続
    pub fn poll_event(&self) -> io::Result<Option<char>> {
        if event::poll(std::time::Duration::from_millis(10))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(Some('q')),
                    KeyCode::Char('i') => return Ok(Some('i')),
                    KeyCode::Char('1') => return Ok(Some('1')),
                    KeyCode::Char('2') => return Ok(Some('2')),
                    KeyCode::Char('3') => return Ok(Some('3')),
                    _ => {}
                }
            }
        }
        Ok(None)
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

    let label = if state.multi_run_mode {
        format!(
            "Multi-Run: {}/{} | Gen {}/{} | Best: {:.4}{}",
            state.completed_runs, state.total_runs,
            state.generation, state.max_generations, state.best_fitness, eta_str
        )
    } else {
        format!(
            "Gen {}/{} | Best: {:.4}{}",
            state.generation, state.max_generations, state.best_fitness, eta_str
        )
    };

    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title(title))
        .gauge_style(Style::default().fg(Color::Cyan).bg(Color::Black))
        .percent((progress * 100.0) as u16)
        .label(label);

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

/// マルチラン用4面デバッグ表示
fn render_multi_run_debug(f: &mut Frame, state: &TuiState) {
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),   // Progress bar
            Constraint::Min(30),     // 4面表示
        ])
        .split(f.area());

    // プログレスバー（マルチラン用）
    let progress_text = format!(
        "Multi-Run: {}/{} completed | Gen: {} | Best: {:.4}",
        state.completed_runs, state.total_runs, state.generation, state.best_fitness
    );
    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("Progress"))
        .gauge_style(Style::default().fg(Color::Cyan))
        .ratio(state.generation as f64 / state.max_generations as f64)
        .label(progress_text);
    f.render_widget(gauge, main_layout[0]);

    // 4面グリッド（2x2）
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_layout[1]);

    let top_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[0]);

    let bottom_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[1]);

    let panels = [top_cols[0], top_cols[1], bottom_cols[0], bottom_cols[1]];

    for (i, &panel_area) in panels.iter().enumerate() {
        if i < state.multi_run_states.len() {
            let (run_id, fitness, ref layout_opt) = &state.multi_run_states[i];
            render_multi_run_panel(f, panel_area, *run_id, *fitness, layout_opt.as_ref());
        } else {
            // 空パネル
            let block = Block::default()
                .borders(Borders::ALL)
                .title(format!("Run {} (waiting)", i));
            f.render_widget(block, panel_area);
        }
    }
}

/// マルチラン用パネル描画
fn render_multi_run_panel(f: &mut Frame, area: Rect, run_id: usize, fitness: f64, layout: Option<&KeyboardLayout>) {
    let title = format!("Run {} | Fitness: {:.4}", run_id, fitness);

    let layout = match layout {
        Some(l) => l,
        None => {
            let block = Block::default()
                .borders(Borders::ALL)
                .title(title);
            f.render_widget(block, area);
            return;
        }
    };

    let mut lines: Vec<Line> = vec![];

    // 4層を圧縮表示（各レイヤー1行）
    let layer_names = ["L0", "L1(☆)", "L2(★)", "L3(◆)"];
    for (layer_idx, layer_name) in layer_names.iter().enumerate() {
        // 中段のみ表示（圧縮のため）
        let row_str: String = layout.layers[layer_idx][1]
            .iter()
            .map(|s| {
                if s == "　" {
                    "□"
                } else if s.chars().count() > 1 {
                    // 2gram は最初の文字のみ
                    &s[..s.chars().next().unwrap().len_utf8()]
                } else {
                    s.as_str()
                }
            })
            .collect::<Vec<_>>()
            .join("");
        lines.push(Line::from(format!("{}: {}", layer_name, row_str)));
    }

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(title))
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}

/// キーボード配列を描画
fn render_keyboard(f: &mut Frame, area: Rect, state: &TuiState) {
    // view_mode: 0=best, 1=2nd_best, 2=initial
    let (layout, title, fitness_str) = match state.view_mode {
        1 => {
            // 2nd best配列を表示
            match &state.second_best_layout {
                Some(l) => (l, "2nd Best Layout", format!("Fitness: {:.4}", state.second_best_fitness)),
                None => {
                    // 2nd bestがない場合はinitialを表示
                    (&state.initial_layout, "2nd Best Layout (none yet)", String::from("(Initial)"))
                }
            }
        }
        2 => {
            // Initial配列を表示
            (&state.initial_layout, "Initial Layout", format!("Fitness: {:.4}", state.initial_layout.fitness))
        }
        _ => {
            // Best配列を表示
            match &state.best_layout {
                Some(l) => (l, "Best Layout", format!("Fitness: {:.4}", state.best_fitness)),
                None => {
                    // best_layoutがない場合はinitialを表示
                    (&state.initial_layout, "Best Layout (none yet)", format!("Fitness: {:.4}", state.initial_layout.fitness))
                }
            }
        }
    };

    let mut lines: Vec<Line> = vec![Line::from(Span::styled(
        format!("{} [Press 'i' to toggle]", fitness_str),
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
    ))];

    lines.push(Line::from(""));

    // 4層すべてを表示
    let layer_names = [
        "Layer 0 (無シフト)",
        "Layer 1 (☆)",
        "Layer 2 (★)",
        "Layer 3 (◆)",
    ];

    for (layer_idx, layer_name) in layer_names.iter().enumerate() {
        lines.push(Line::from(Span::styled(
            format!("{}:", layer_name),
            Style::default().fg(Color::Cyan),
        )));

        for row in 0..3 {
            let row_str: String = layout.layers[layer_idx][row]
                .iter()
                .map(|s| if s == "　" { "□".to_string() } else { s.clone() })
                .collect::<Vec<_>>()
                .iter()
                .map(|s| format!("{} ", s))
                .collect();
            lines.push(Line::from(format!("  {}", row_str)));
        }
        lines.push(Line::from(""));
    }

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title),
    );

    f.render_widget(paragraph, area);
}

/// Colemak一致詳細を計算（評価関数と同じロジック）
/// 戻り値: (perfect, partial, total, layer別perfect配列)
fn calc_colemak_match_detail(layers: &Vec<Vec<Vec<String>>>) -> (usize, usize, usize, [usize; 4]) {
    use crate::layout::{romaji_phonemes, COLEMAK_POSITIONS, cols_for_row};
    use std::collections::HashMap;

    // COLEMAK_POSITIONSから音素→位置のマップを作成
    let mut phoneme_pos: HashMap<&str, (usize, usize)> = HashMap::new();
    for &(phoneme, row, col) in COLEMAK_POSITIONS {
        phoneme_pos.insert(phoneme, (row, col));
    }

    let mut perfect = 0; // 完全一致（両音素とも位置一致）
    let mut partial = 0; // 部分一致（片方のみ一致または行/手一致）
    let mut total = 0;
    let mut layer_perfect = [0usize; 4]; // 各レイヤーのperfect数

    // 全4レイヤーを評価
    for layer in 0..4.min(layers.len()) {
        for row in 0..3 {
            let cols = cols_for_row(row);
            for col in 0..cols {
                let s = &layers[layer][row][col];
                let c = match s.chars().next() {
                    Some(ch) => ch,
                    None => continue,
                };

                // 除外文字
                if matches!(
                    c,
                    'A' | 'B' | 'C' | 'D' | '☆' | '★' | '◎' | '◆' | '、' | '。' | '　'
                        | 'ー' | 'っ' | 'ゃ' | 'ゅ' | 'ょ' | 'ぁ' | 'ぃ' | 'ぅ' | 'ぇ'
                        | 'ぉ' | '゛' | '゜'
                ) {
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
                            cons_score = 1.0; // 完全一致
                        } else if row == exp_row {
                            cons_score = 0.5; // 行一致
                        } else if (col < 5 && exp_col < 5) || (col >= 5 && exp_col >= 5) {
                            cons_score = 0.25; // 手一致
                        }
                    }
                }

                // 母音チェック
                if let Some(vow) = vowel {
                    if let Some(&(exp_row, exp_col)) = phoneme_pos.get(vow) {
                        if row == exp_row && col == exp_col {
                            vowel_score = 1.0; // 完全一致
                        } else if row == exp_row {
                            vowel_score = 0.5; // 行一致
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
                        layer_perfect[layer] += 1;
                    } else if vowel_score >= 0.5 {
                        partial += 1;
                    }
                } else if is_consonant_only {
                    // 子音のみ（ん）：完全一致なら◎、行一致以上で○
                    if cons_score >= 1.0 {
                        perfect += 1;
                        layer_perfect[layer] += 1;
                    } else if cons_score >= 0.5 {
                        partial += 1;
                    }
                } else {
                    // 子音+母音：両方完全一致で◎、どちらか完全一致で○
                    let has_perfect_match = cons_score >= 1.0 || vowel_score >= 1.0;
                    let total_score = cons_score + vowel_score;

                    if total_score >= 1.8 {
                        perfect += 1;
                        layer_perfect[layer] += 1;
                    } else if has_perfect_match {
                        partial += 1;
                    }
                }
            }
        }
    }

    (perfect, partial, total, layer_perfect)
}

/// 月配列一致詳細を計算（ヘルパー関数）
/// 戻り値: 各レイヤーの(一致数, 総数)の配列 [4]
fn calc_tsuki_match_detail(
    layers: &Vec<Vec<Vec<String>>>,
) -> [(usize, usize); 4] {
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

    let mut result = [(0usize, 0usize); 4];

    for ga_layer in 0..4.min(layers.len()) {
        // GA Layer 0 → 月 Layer 0（表面）
        // GA Layer 1,2,3 → 月 Layer 1（裏面）
        let tsuki_layer = if ga_layer == 0 { 0 } else { 1 };

        for row in 0..3 {
            // 月配列は10列のみなので、比較は10列まで
            for col in 0..10 {
                let s = &layers[ga_layer][row][col];
                let kana = match s.chars().next() {
                    Some(ch) => ch,
                    None => continue,
                };
                let tsuki_char = tsuki_layers[tsuki_layer][row][col];

                if matches!(
                    kana,
                    'A' | 'B' | 'C' | 'D' | '★' | '☆' | '◎' | '◆' | '、' | '。' | '　'
                ) || matches!(
                    tsuki_char,
                    '★' | '☆' | '、' | '。' | '゛' | '゜' | '　'
                ) {
                    continue;
                }

                result[ga_layer].1 += 1; // total
                if kana == tsuki_char {
                    result[ga_layer].0 += 1; // matched
                }
            }
        }
    }
    result
}

/// キーごとの位置コストを計算（評価ロジックと同じ）
fn calc_position_cost_for_key(pos: &crate::layout::KeyPos) -> f64 {
    // POSITION_COSTS配列から直接取得（evaluation.rsと完全に同じ）
    crate::layout::get_position_cost(pos.layer, pos.row, pos.col)
}

/// デバッグパネルを描画（全計算過程・3カラム）
fn render_debug_panel(f: &mut Frame, area: Rect, state: &TuiState) {
    // 表示中のパネル数に応じて幅を動的に調整
    let visible_count = state.debug_panel_visible.iter().filter(|&&v| v).count();

    if visible_count == 0 {
        // 全非表示の場合は説明を表示
        let help_text = vec![
            Line::from(""),
            Line::from("全パネルが非表示です。表示するには:"),
            Line::from(""),
            Line::from("  [1] Scores+計算式"),
            Line::from("  [2] キーごと採点"),
            Line::from("  [3] 位置コスト"),
            Line::from(""),
            Line::from("数字キー 1/2/3 で表示・非表示を切り替え"),
        ];
        let paragraph = Paragraph::new(help_text)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Debug Panel"));
        f.render_widget(paragraph, area);
        return;
    }

    // 表示するパネルのConstraintを動的に構築
    let mut constraints = Vec::new();
    let width_per_panel = 100 / visible_count as u16;

    for &visible in &state.debug_panel_visible {
        if visible {
            constraints.push(Constraint::Percentage(width_per_panel));
        }
    }

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area);
    
    let mut left_lines = vec![];
    let mut center_lines = vec![];
    let mut right_lines = vec![];
    
    // view_mode: 0=best, 1=2nd_best, 2=initial
    let layout = match state.view_mode {
        1 => state.second_best_layout.as_ref().unwrap_or(&state.initial_layout),
        2 => &state.initial_layout,
        _ => state.best_layout.as_ref().unwrap_or(&state.initial_layout),
    };

    if let Some(w) = &state.weights {
        let s = &layout.scores;
        let layers = &layout.layers;
        
        // ========== 左カラム: Similarity & Scores + 計算式 ==========
        left_lines.push(Line::from(Span::styled(
            "=== Similarity ===",
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        )));
        
        // Similarity詳細（全5レイヤー）
        let (colemak_perfect, colemak_partial, colemak_total, colemak_by_layer) = calc_colemak_match_detail(&layers);
        left_lines.push(Line::from(format!(
            "Colemak: {:.1}% (◎{} ○{} ×{})",
            s.colemak_similarity, colemak_perfect, colemak_partial,
            colemak_total - colemak_perfect - colemak_partial
        )));
        left_lines.push(Line::from(format!(
            "  L0:{} L1:{} L2:{} L3:{}",
            colemak_by_layer[0], colemak_by_layer[1], colemak_by_layer[2],
            colemak_by_layer[3]
        )));

        let tsuki_detail = calc_tsuki_match_detail(&layers);
        let total_match: usize = tsuki_detail.iter().map(|(m, _)| m).sum();
        let total_all: usize = tsuki_detail.iter().map(|(_, t)| t).sum();
        left_lines.push(Line::from(format!(
            "月配列: {:.1}% (○{}/{})",
            s.tsuki_similarity, total_match, total_all
        )));
        left_lines.push(Line::from(format!(
            "  L0:{}/{} L1:{}/{} L2:{}/{} L3:{}/{}",
            tsuki_detail[0].0, tsuki_detail[0].1,
            tsuki_detail[1].0, tsuki_detail[1].1,
            tsuki_detail[2].0, tsuki_detail[2].1,
            tsuki_detail[3].0, tsuki_detail[3].1
        )));
        
        left_lines.push(Line::from(""));
        left_lines.push(Line::from(Span::styled(
            "=== Core Metrics ===",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )));

        // Core計算式（基本6指標）- .max(0.01)で0除算防止
        let sf = (s.same_finger/100.0).max(0.01);
        let rs = (s.row_skip/100.0).max(0.01);
        let hp = (s.home_position/100.0).max(0.01);
        let tk = (s.total_keystrokes/100.0).max(0.01);
        let alt = (s.alternating/100.0).max(0.01);
        let pc = (s.position_cost/100.0).max(0.01);

        // 対数空間で計算（evaluation.rsと同じ）
        let sf_powered = (w.same_finger * sf.ln()).exp();
        let rs_powered = (w.row_skip * rs.ln()).exp();
        let hp_powered = (w.home_position * hp.ln()).exp();
        let tk_powered = (w.total_keystrokes * tk.ln()).exp();
        let alt_powered = (w.alternating * alt.ln()).exp();
        let pc_powered = (w.position_cost * pc.ln()).exp();

        left_lines.push(Line::from(format!("同指連続低: {:.3}^{:.0}={:.2e}",
            sf, w.same_finger, sf_powered)));
        left_lines.push(Line::from(format!("段越え低: {:.3}^{:.0}={:.2e}",
            rs, w.row_skip, rs_powered)));
        left_lines.push(Line::from(format!("ホーム率: {:.3}^{:.0}={:.2e} (L0:1/L1,2:0.1/L3:除外)",
            hp, w.home_position, hp_powered)));
        left_lines.push(Line::from(format!("打鍵少: {:.3}^{:.0}={:.2e}",
            tk, w.total_keystrokes, tk_powered)));
        left_lines.push(Line::from(format!("左右交互: {:.3}^{:.0}={:.2e}",
            alt, w.alternating, alt_powered)));
        left_lines.push(Line::from(format!("位置コスト: {:.3}^{:.0}={:.2e} (線形)",
            pc, w.position_cost, pc_powered)));

        let total_weight = w.same_finger + w.row_skip + w.home_position
            + w.total_keystrokes + w.alternating + w.position_cost;

        let log_core_sum = w.same_finger * sf.ln()
            + w.row_skip * rs.ln()
            + w.home_position * hp.ln()
            + w.total_keystrokes * tk.ln()
            + w.alternating * alt.ln()
            + w.position_cost * pc.ln();
        let core_mult = (log_core_sum / total_weight).exp() * 100.0;
        
        left_lines.push(Line::from(""));
        left_lines.push(Line::from(format!("log_sum={:.2}, 重み計={:.0}", log_core_sum, total_weight)));
        left_lines.push(Line::from(format!("→Core: exp({:.2}/{:.0})*100={:.4}",
            log_core_sum, total_weight, core_mult)));
        
        // Bonus計算式
        left_lines.push(Line::from(""));
        left_lines.push(Line::from(Span::styled(
            "=== Bonus ===",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )));
        left_lines.push(Line::from(format!("単打率: {:.1}×{:.1}={:.1}",
            s.single_key, w.single_key, s.single_key * w.single_key)));
        left_lines.push(Line::from(format!("リダイレクト低: {:.1}×{:.1}={:.1}",
            s.redirect_low, w.redirect_low, s.redirect_low * w.redirect_low)));
        left_lines.push(Line::from(format!("月類似: {:.1}×{:.1}={:.1}",
            s.tsuki_similarity, w.tsuki_similarity, s.tsuki_similarity * w.tsuki_similarity)));
        left_lines.push(Line::from(format!("Colemak: {:.1}×{:.1}={:.1}",
            s.colemak_similarity, w.colemak_similarity, s.colemak_similarity * w.colemak_similarity)));
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
            + s.redirect_low * w.redirect_low
            + s.tsuki_similarity * w.tsuki_similarity
            + s.colemak_similarity * w.colemak_similarity
            + s.roll * w.roll
            + s.inroll * w.inroll
            + s.arpeggio * w.arpeggio
            + s.memorability * w.memorability
            + s.shift_balance * w.shift_balance;
        let bonus_scale = (w.single_key
            + w.redirect_low + w.tsuki_similarity + w.colemak_similarity + w.roll
            + w.inroll + w.arpeggio + w.memorability + w.shift_balance) * 100.0;
        
        left_lines.push(Line::from(""));
        left_lines.push(Line::from(format!("→Bonus: {:.2}/{:.0}={:.4}",
            additive_bonus, bonus_scale, additive_bonus/bonus_scale)));
        
        // 最終Fitness
        let final_fitness = core_mult * (1.0 + additive_bonus / bonus_scale);
        left_lines.push(Line::from(""));
        left_lines.push(Line::from(Span::styled(
            format!("■ Final(計算): {:.4}", final_fitness),
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        )));
        left_lines.push(Line::from(format!("{:.2}×(1+{:.4})={:.4}",
            core_mult, additive_bonus/bonus_scale, final_fitness)));
        // 実際に保存されているfitness値との比較
        left_lines.push(Line::from(Span::styled(
            format!("■ Stored fitness: {:.4}", layout.fitness),
            Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
        )));
        if (final_fitness - layout.fitness).abs() > 0.01 {
            left_lines.push(Line::from(Span::styled(
                format!("⚠ 差分: {:.4}", final_fitness - layout.fitness),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )));
        }
        
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
        
        for layer in 0..4.min(layers.len()) {
            let layer_name = match layer {
                0 => "L0",
                1 => "L1",
                2 => "L2",
                3 => "L3",
                _ => "",
            };
            right_lines.push(Line::from(format!("{}:", layer_name)));

            for row in 0..3 {
                let mut row_line = String::new();

                for col in 0..10 {
                    let kana_str = &layers[layer][row][col];
                    let kana_char = kana_str.chars().next().unwrap_or('　');
                    let pos = KeyPos::new(layer, row, col);
                    let cost = calc_position_cost_for_key(&pos);

                    // 頻度取得（1-gramから）
                    let freq = freq_map.get(&kana_char).copied().unwrap_or(0.0);

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
                    row_line.push_str(&format!("{}:{}({:>2}) ", kana_str, freq_str, display_cost));
                }

                right_lines.push(Line::from(format!(" {}", row_line)));
            }
            right_lines.push(Line::from(""));
        }

        right_lines.push(Line::from("L0: 通常 L1:☆ L2:★ L3:◆"));
        right_lines.push(Line::from("L1: ☆(col7)上下Ver+27,Out+9"));
        right_lines.push(Line::from("L2: ★(col2)上下Ver+27,Out+9"));
        right_lines.push(Line::from("L3: ◆シフト (同L0基準)"));
        
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
        for layer in 0..3.min(layers.len()) {
            let layer_name = match layer {
                0 => "L0:",
                1 => "L1:",
                2 => "L2:",
                _ => "",
            };
            center_lines.push(Line::from(layer_name));

            for row in 0..3 {
                let mut match_line = String::new();
                for col in 0..10 {
                    let s = &layers[layer][row][col];
                    let c = match s.chars().next() {
                        Some(ch) => ch,
                        None => {
                            match_line.push_str("  ");
                            continue;
                        }
                    };

                    if matches!(
                        c,
                        'A' | 'B' | 'C' | 'D' | '☆' | '★' | '◎' | '◆' | '、' | '。' | '　'
                            | 'ー' | 'っ' | 'ゃ' | 'ゅ' | 'ょ' | 'ぁ' | 'ぃ' | 'ぅ' | 'ぇ'
                            | 'ぉ' | '゛' | '゜'
                    ) {
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
        
        for layer in 0..2.min(layers.len()) {
            let layer_name = match layer {
                0 => "L0",
                1 => "L1",
                _ => "",
            };
            center_lines.push(Line::from(format!("{}:", layer_name)));

            for row in 0..3 {
                let mut match_line = String::new();
                for col in 0..10 {
                    let kana_str = &layers[layer][row][col];
                    let kana = kana_str.chars().next().unwrap_or('　');
                    let tsuki_char = tsuki_layers[layer][row][col];

                    if matches!(
                        kana,
                        'A' | 'B' | 'C' | 'D' | '★' | '☆' | '◎' | '◆' | '、' | '。' | '　'
                    ) || matches!(
                        tsuki_char,
                        '★' | '☆' | '、' | '。' | '゛' | '゜' | '　'
                    ) {
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

    // 表示中のパネルのみを描画（列インデックスを動的に割り当て）
    let mut col_idx = 0;

    // パネル1: Scores+計算式
    if state.debug_panel_visible[0] {
        let left_para = Paragraph::new(left_lines)
            .block(Block::default()
                .title("[1] Scores+計算式")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)))
            .wrap(Wrap { trim: false });
        f.render_widget(left_para, columns[col_idx]);
        col_idx += 1;
    }

    // パネル2: キーごと採点
    if state.debug_panel_visible[1] {
        let center_para = Paragraph::new(center_lines)
            .block(Block::default()
                .title("[2] キーごと採点")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)))
            .wrap(Wrap { trim: false });
        f.render_widget(center_para, columns[col_idx]);
        col_idx += 1;
    }

    // パネル3: 位置コスト
    if state.debug_panel_visible[2] {
        let right_para = Paragraph::new(right_lines)
            .block(Block::default()
                .title("[3] 位置コスト")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)))
            .wrap(Wrap { trim: false });
        f.render_widget(right_para, columns[col_idx]);
    }
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
    // Colemak一致率の詳細計算（全4レイヤー）
    let (colemak_perfect, colemak_partial, colemak_total, colemak_by_layer) = calc_colemak_match_detail(&layout.layers);
    lines.push(Line::from(format!(
        "Colemak:    {:.1}% (◎{} ○{} ×{})",
        s.colemak_similarity, colemak_perfect, colemak_partial,
        colemak_total - colemak_perfect - colemak_partial
    )));
    lines.push(Line::from(format!(
        "  L0:{} L1:{} L2:{} L3:{}",
        colemak_by_layer[0], colemak_by_layer[1], colemak_by_layer[2],
        colemak_by_layer[3]
    )));

    // 月配列一致率の詳細計算（全4レイヤー）
    let tsuki_detail = calc_tsuki_match_detail(&layout.layers);
    let tsuki_match: usize = tsuki_detail.iter().map(|(m, _)| m).sum();
    let tsuki_total: usize = tsuki_detail.iter().map(|(_, t)| t).sum();
    lines.push(Line::from(format!(
        "月配列:     {:.1}% (○{} ×{})",
        s.tsuki_similarity, tsuki_match, tsuki_total - tsuki_match
    )));
    lines.push(Line::from(format!(
        "  L0:{}/{} L1:{}/{} L2:{}/{} L3:{}/{}",
        tsuki_detail[0].0, tsuki_detail[0].1,
        tsuki_detail[1].0, tsuki_detail[1].1,
        tsuki_detail[2].0, tsuki_detail[2].1,
        tsuki_detail[3].0, tsuki_detail[3].1
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
            "ホーム率:   {:.1}% ^{:.1} (L0:1/L1,2:0.1/L3:除外)",
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
        lines.push(Line::from(format!(
            "位置コスト: {:.1}% ^{:.1}",
            s.position_cost, weights.position_cost
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
            "Colemak:    {:.1}% ×{:.1}",
            s.colemak_similarity, weights.colemak_similarity
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
                Ok(Some('q')) => {
                    let mut state = state.lock().unwrap();
                    state.running = false;
                    break;
                }
                Ok(Some('i')) => {
                    let mut state = state.lock().unwrap();
                    state.view_mode = (state.view_mode + 1) % 3;  // 0=best, 1=2nd_best, 2=initial
                }
                Ok(Some('1')) => {
                    let mut state = state.lock().unwrap();
                    state.debug_panel_visible[0] = !state.debug_panel_visible[0];
                }
                Ok(Some('2')) => {
                    let mut state = state.lock().unwrap();
                    state.debug_panel_visible[1] = !state.debug_panel_visible[1];
                }
                Ok(Some('3')) => {
                    let mut state = state.lock().unwrap();
                    state.debug_panel_visible[2] = !state.debug_panel_visible[2];
                }
                Ok(None) => {}
                Err(e) => {
                    eprintln!("TUI event error: {}", e);
                    break;
                }
                _ => {}
            }

            // ETA表示のため1秒に1回更新（デバッグモードは負荷が高いため少し長め）
            let state_for_sleep = state.lock().unwrap();
            let sleep_ms = if state_for_sleep.debug { 1000 } else { 500 };
            drop(state_for_sleep);
            std::thread::sleep(std::time::Duration::from_millis(sleep_ms));
        }

        if let Err(e) = app.cleanup() {
            eprintln!("TUI cleanup error: {}", e);
        }
    })
}
