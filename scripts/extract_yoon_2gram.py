#!/usr/bin/env python3
"""
2gramから拗音終わりの2文字を抽出するスクリプト
"""

import sys
from pathlib import Path

def is_yoon(c):
    """拗音（小書き文字）かどうか"""
    return c in 'ゃゅょゎァィゥェォヵヶャュョヮ'

def extract_yoon_2grams(ngram2_path, output_path=None):
    """2gramから拗音終わりを抽出"""
    ngram2_path = Path(ngram2_path)

    if output_path is None:
        output_path = ngram2_path.parent / "2gram_yoon.txt"

    yoon_2grams = []

    with open(ngram2_path, 'r', encoding='utf-8') as f:
        for line in f:
            parts = line.strip().split('\t')
            if len(parts) >= 2:
                count = int(parts[0])
                ngram = parts[1]

                # 2文字で、拗音で終わるものだけ抽出
                if len(ngram) == 2 and is_yoon(ngram[1]):
                    yoon_2grams.append((count, ngram))

    # 出力
    with open(output_path, 'w', encoding='utf-8') as f:
        for count, ngram in yoon_2grams:
            f.write(f"{count}\t{ngram}\n")

    print(f"拗音終わり2gram: {len(yoon_2grams)}種類")
    print(f"出力: {output_path}")

    # トップ20を表示
    print("\nトップ20:")
    for i, (count, ngram) in enumerate(yoon_2grams[:20], 1):
        print(f"{i:2d}. {ngram:3s} ({count:5d})")

    return yoon_2grams

if __name__ == '__main__':
    if len(sys.argv) < 2:
        print("使用方法: python scripts/extract_yoon_2gram.py 2gram.txt")
        sys.exit(1)

    extract_yoon_2grams(sys.argv[1])
