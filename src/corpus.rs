//! コーパスモジュール
//! 
//! N-gramデータの読み込みと管理を行う。

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// コーパス統計データ
#[derive(Clone, Debug)]
pub struct CorpusStats {
    /// 1-gram（文字）頻度
    pub char_freq: HashMap<char, usize>,
    /// 2-gram（連続2文字）頻度
    pub bigram_freq: HashMap<(char, char), usize>,
    /// 3-gram（連続3文字）頻度
    pub trigram_freq: HashMap<(char, char, char), usize>,
    /// 4-gram（連続4文字）頻度
    pub fourgram_freq: HashMap<(char, char, char, char), usize>,
    /// ひらがな文字の頻度順リスト（1gramから生成）
    pub hiragana_by_freq: Vec<char>,
}

/// ひらがな文字かどうかを判定
fn is_hiragana(c: char) -> bool {
    matches!(c, 'ぁ'..='ん' | 'ー' | 'ゔ')
}

impl CorpusStats {
    /// 空のコーパス統計を作成
    pub fn new() -> Self {
        Self {
            char_freq: HashMap::new(),
            bigram_freq: HashMap::new(),
            trigram_freq: HashMap::new(),
            fourgram_freq: HashMap::new(),
            hiragana_by_freq: Vec::new(),
        }
    }

    /// N-gramファイルからコーパス統計を読み込む
    /// 
    /// ファイル形式: `count\tcharacters\tn` (タブ区切り)
    /// - count: 出現回数
    /// - characters: 文字列（1-gram〜4-gram）
    /// - n: N-gramのN値
    /// 
    /// `〓` は改行を表すノイズとして除外される。
    /// ひらがな文字は頻度順にソートして `hiragana_by_freq` に格納される。
    pub fn from_ngram_files(
        gram1_path: Option<&Path>,
        gram2_path: Option<&Path>,
        gram3_path: Option<&Path>,
        gram4_path: Option<&Path>,
    ) -> Result<Self, std::io::Error> {
        let mut stats = Self::new();
        
        // 1-gramと頻度順ひらがなリストを同時に構築
        let mut hiragana_freq: Vec<(char, usize)> = Vec::new();

        // 1-gram
        if let Some(path) = gram1_path {
            let file = File::open(path)?;
            let reader = BufReader::new(file);
            for line in reader.lines() {
                let line = line?;
                if let Some((count, chars)) = Self::parse_ngram_line(&line) {
                    let c: Vec<char> = chars.chars().collect();
                    if c.len() == 1 && c[0] != '〓' {
                        let ch = c[0];
                        stats.char_freq.insert(ch, count);
                        
                        // ひらがなのみを頻度リストに追加
                        if is_hiragana(ch) && ch != '、' && ch != '。' {
                            hiragana_freq.push((ch, count));
                        }
                    }
                }
            }
        }
        
        // 頻度順でソート（降順）
        hiragana_freq.sort_by(|a, b| b.1.cmp(&a.1));
        stats.hiragana_by_freq = hiragana_freq.into_iter().map(|(c, _)| c).collect();

        // 2-gram
        if let Some(path) = gram2_path {
            let file = File::open(path)?;
            let reader = BufReader::new(file);
            for line in reader.lines() {
                let line = line?;
                if let Some((count, chars)) = Self::parse_ngram_line(&line) {
                    let c: Vec<char> = chars.chars().collect();
                    if c.len() == 2 && !c.contains(&'〓') {
                        stats.bigram_freq.insert((c[0], c[1]), count);
                    }
                }
            }
        }

        // 3-gram
        if let Some(path) = gram3_path {
            let file = File::open(path)?;
            let reader = BufReader::new(file);
            for line in reader.lines() {
                let line = line?;
                if let Some((count, chars)) = Self::parse_ngram_line(&line) {
                    let c: Vec<char> = chars.chars().collect();
                    if c.len() == 3 && !c.contains(&'〓') {
                        stats.trigram_freq.insert((c[0], c[1], c[2]), count);
                    }
                }
            }
        }

        // 4-gram
        if let Some(path) = gram4_path {
            let file = File::open(path)?;
            let reader = BufReader::new(file);
            for line in reader.lines() {
                let line = line?;
                if let Some((count, chars)) = Self::parse_ngram_line(&line) {
                    let c: Vec<char> = chars.chars().collect();
                    if c.len() == 4 && !c.contains(&'〓') {
                        stats.fourgram_freq.insert((c[0], c[1], c[2], c[3]), count);
                    }
                }
            }
        }

        // 記号（;・）を最低頻度として追加
        stats.add_symbol_chars();

        Ok(stats)
    }

    /// N-gram行をパース
    /// 形式: `count\tcharacters\tn`
    fn parse_ngram_line(line: &str) -> Option<(usize, String)> {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 2 {
            if let Ok(count) = parts[0].parse::<usize>() {
                return Some((count, parts[1].to_string()));
            }
        }
        None
    }

    /// テキストファイルからコーパス統計を計算
    pub fn from_text(text: &str) -> Self {
        let mut stats = Self::new();
        let chars: Vec<char> = text.chars().collect();

        // 1-gram
        for &c in &chars {
            *stats.char_freq.entry(c).or_insert(0) += 1;
        }

        // 2-gram
        for window in chars.windows(2) {
            let key = (window[0], window[1]);
            *stats.bigram_freq.entry(key).or_insert(0) += 1;
        }

        // 3-gram
        for window in chars.windows(3) {
            let key = (window[0], window[1], window[2]);
            *stats.trigram_freq.entry(key).or_insert(0) += 1;
        }

        // 4-gram
        for window in chars.windows(4) {
            let key = (window[0], window[1], window[2], window[3]);
            *stats.fourgram_freq.entry(key).or_insert(0) += 1;
        }

        // ひらがな頻度順リストを構築
        let mut hiragana_freq: Vec<(char, usize)> = stats
            .char_freq
            .iter()
            .filter(|(&c, _)| is_hiragana(c) && c != '、' && c != '。')
            .map(|(&c, &count)| (c, count))
            .collect();
        hiragana_freq.sort_by(|a, b| b.1.cmp(&a.1));
        stats.hiragana_by_freq = hiragana_freq.into_iter().map(|(c, _)| c).collect();

        // 記号（;・）を最低頻度として追加
        stats.add_symbol_chars();
        
        stats
    }

    /// テキストファイルを読み込んでコーパス統計を計算
    pub fn from_text_file(path: &Path) -> Result<Self, std::io::Error> {
        let content = std::fs::read_to_string(path)?;
        Ok(Self::from_text(&content))
    }
    
    /// 記号文字（;・）をコーパスに追加（評価用、配置対象外）
    fn add_symbol_chars(&mut self) {
        // Layer 1固定記号をchar_freqに追加（評価で使用）
        for &c in &['；', '・'] {
            if !self.char_freq.contains_key(&c) {
                self.char_freq.insert(c, 1);
            }
        }
        
        // hiragana_by_freqには追加しない（固定位置なので配置対象外）
    }

    /// コーパスの総文字数
    pub fn total_chars(&self) -> usize {
        self.char_freq.values().sum()
    }

    /// コーパスの総2-gram数
    pub fn total_bigrams(&self) -> usize {
        self.bigram_freq.values().sum()
    }

    /// コーパスの総3-gram数
    pub fn total_trigrams(&self) -> usize {
        self.trigram_freq.values().sum()
    }

    /// 統計情報のサマリーを表示
    pub fn summary(&self) -> String {
        format!(
            "Corpus Stats:\n  1-gram types: {}\n  2-gram types: {}\n  3-gram types: {}\n  4-gram types: {}\n  Total chars: {}\n  Hiragana chars: {}",
            self.char_freq.len(),
            self.bigram_freq.len(),
            self.trigram_freq.len(),
            self.fourgram_freq.len(),
            self.total_chars(),
            self.hiragana_by_freq.len()
        )
    }
    
    /// ひらがな頻度順リストを取得（句読点除く）
    pub fn get_hiragana_by_freq(&self) -> &[char] {
        &self.hiragana_by_freq
    }
}

impl Default for CorpusStats {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_text() {
        let text = "あいうえお";
        let stats = CorpusStats::from_text(text);
        
        assert_eq!(stats.char_freq.len(), 5);
        assert_eq!(stats.bigram_freq.len(), 4);
        assert_eq!(stats.trigram_freq.len(), 3);
    }

    #[test]
    fn test_parse_ngram_line() {
        let line = "1234\tあい\t2";
        let result = CorpusStats::parse_ngram_line(line);
        assert_eq!(result, Some((1234, "あい".to_string())));
    }
}
