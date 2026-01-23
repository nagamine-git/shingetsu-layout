#!/usr/bin/env python3
"""
コーパスからN-gramファイルを生成するスクリプト

使用方法:
    python scripts/create_ngram.py corpus_1m.txt

出力:
    1gram.txt, 2gram.txt, 3gram.txt, 4gram.txt
"""

import sys
import argparse
from collections import Counter
from pathlib import Path


def is_hiragana(c):
    """ひらがな・句読点・長音符かどうか"""
    return '\u3040' <= c <= '\u309f' or c in '。、ー'


def filter_hiragana(text):
    """ひらがな・句読点のみを抽出"""
    return ''.join(c for c in text if is_hiragana(c))


def create_ngrams(corpus_path, output_dir=None, min_count=1):
    """N-gramファイルを生成"""
    corpus_path = Path(corpus_path)
    output_dir = Path(output_dir) if output_dir else corpus_path.parent

    print(f"コーパス: {corpus_path}")

    # コーパス読み込み
    with open(corpus_path, 'r', encoding='utf-8') as f:
        text = f.read()

    # ひらがなのみ抽出
    text = filter_hiragana(text.replace('\n', ''))
    print(f"文字数: {len(text):,}")

    # N-gramカウント
    counters = {
        1: Counter(),
        2: Counter(),
        3: Counter(),
        4: Counter(),
    }

    for i in range(len(text)):
        for n in [1, 2, 3, 4]:
            if i + n <= len(text):
                ngram = text[i:i+n]
                counters[n][ngram] += 1

    # 出力
    for n, counter in counters.items():
        output_path = output_dir / f"{n}gram.txt"

        # 頻度でソート（降順）
        sorted_items = sorted(counter.items(), key=lambda x: -x[1])

        # min_count以上のみ出力
        filtered = [(ngram, count) for ngram, count in sorted_items if count >= min_count]

        with open(output_path, 'w', encoding='utf-8') as f:
            for ngram, count in filtered:
                f.write(f"{count}\t{ngram}\t{n}\n")

        print(f"{n}-gram: {len(filtered):,}種類 → {output_path}")

    print("\n完了!")


def main():
    parser = argparse.ArgumentParser(description='コーパスからN-gramを生成')
    parser.add_argument('corpus', help='コーパスファイル')
    parser.add_argument('-o', '--output', help='出力ディレクトリ（デフォルト: コーパスと同じ）')
    parser.add_argument('-m', '--min-count', type=int, default=1, help='最小出現回数（デフォルト: 1）')

    args = parser.parse_args()
    create_ngrams(args.corpus, args.output, args.min_count)


if __name__ == '__main__':
    main()
