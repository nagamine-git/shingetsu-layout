#!/usr/bin/env python3
"""
1gramと拗音終わり2gramをマージして頻度順リストを作成
"""

import sys
from pathlib import Path

def merge_frequencies(ngram1_path, yoon2_path, output_path=None):
    """1gramと拗音2gramをマージ"""
    ngram1_path = Path(ngram1_path)
    yoon2_path = Path(yoon2_path)

    if output_path is None:
        output_path = ngram1_path.parent / "merged_freq.txt"

    # 1gramを読み込み
    chars_1gram = []
    with open(ngram1_path, 'r', encoding='utf-8') as f:
        for line in f:
            parts = line.strip().split('\t')
            if len(parts) >= 2:
                count = int(parts[0])
                char = parts[1]
                # 拗音単体（ょ、ゅ、ゃなど）は除外（これらは2gramの一部として使う）
                if char not in 'ょゅゃゎ':
                    chars_1gram.append((count, char, 1))

    # 拗音2gramを読み込み
    chars_2gram = []
    with open(yoon2_path, 'r', encoding='utf-8') as f:
        for line in f:
            parts = line.strip().split('\t')
            if len(parts) >= 2:
                count = int(parts[0])
                ngram = parts[1]
                chars_2gram.append((count, ngram, 2))

    # マージして頻度順にソート
    all_chars = chars_1gram + chars_2gram
    all_chars.sort(key=lambda x: -x[0])

    # 出力
    with open(output_path, 'w', encoding='utf-8') as f:
        for count, char, n in all_chars:
            f.write(f"{count}\t{char}\t{n}\n")

    print(f"1gram: {len(chars_1gram)}文字")
    print(f"拗音2gram: {len(chars_2gram)}文字")
    print(f"合計: {len(all_chars)}文字")
    print(f"出力: {output_path}")

    # トップ50を表示
    print("\nトップ50（頻度順）:")
    for i, (count, char, n) in enumerate(all_chars[:50], 1):
        char_type = "2g" if n == 2 else "1g"
        print(f"{i:2d}. {char:3s} ({count:6d}) [{char_type}]")

    return all_chars

if __name__ == '__main__':
    if len(sys.argv) < 3:
        print("使用方法: python scripts/merge_1gram_yoon2gram.py 1gram.txt 2gram_yoon.txt")
        sys.exit(1)

    merge_frequencies(sys.argv[1], sys.argv[2])
