#!/usr/bin/env python3
"""
keyboard_analyzer形式のJSONから各種フォーマットを生成する

- hazkey用テーブル (TSV) - ANSI配列対応
- Karabiner Elements用JSON
- QWERTY/Colemak配列対応
"""

import json
import sys
from pathlib import Path
from typing import Dict, List, Tuple, Any


# QWERTY → Colemak 変換マップ
QWERTY_TO_COLEMAK = {
    'q': 'q', 'w': 'w', 'e': 'f', 'r': 'p', 't': 'g',
    'y': 'j', 'u': 'l', 'i': 'u', 'o': 'y', 'p': ';',
    'a': 'a', 's': 'r', 'd': 's', 'f': 't', 'g': 'd',
    'h': 'h', 'j': 'n', 'k': 'e', 'l': 'i', ';': 'o',
    'z': 'z', 'x': 'x', 'c': 'c', 'v': 'v', 'b': 'b',
    'n': 'k', 'm': 'm', ',': ',', '.': '.', '/': '/',
    '[': '[', ']': ']', "'": "'", '-': '-', '=': '=',
    '\\': '\\', 'space': 'space'
}

# 濁音変換 (1回目の゛)
DAKUTEN_MAP = {
    'か': 'が', 'き': 'ぎ', 'く': 'ぐ', 'け': 'げ', 'こ': 'ご',
    'さ': 'ざ', 'し': 'じ', 'す': 'ず', 'せ': 'ぜ', 'そ': 'ぞ',
    'た': 'だ', 'ち': 'ぢ', 'つ': 'づ', 'て': 'で', 'と': 'ど',
    'は': 'ば', 'ひ': 'び', 'ふ': 'ぶ', 'へ': 'べ', 'ほ': 'ぼ',
    'う': 'ゔ',
}

# 半濁音変換 (清音から)
HANDAKUTEN_MAP = {
    'は': 'ぱ', 'ひ': 'ぴ', 'ふ': 'ぷ', 'へ': 'ぺ', 'ほ': 'ぽ',
}

# 濁音→半濁音変換 (2回目の゛で濁音から半濁音へ)
DAKUTEN_TO_HANDAKUTEN_MAP = {
    'ば': 'ぱ', 'び': 'ぴ', 'ぶ': 'ぷ', 'べ': 'ぺ', 'ぼ': 'ぽ',
}

# 母音→小書き変換 (2回目の゛で小書きへ) ※うは除外(う→ゔがあるため)
VOWEL_TO_KOGAKI_MAP = {
    'あ': 'ぁ', 'い': 'ぃ', 'え': 'ぇ', 'お': 'ぉ',
}

# ゔ→ぅ (2回目の゛で小書き)
VU_TO_KOGAKI_MAP = {
    'ゔ': 'ぅ',
}

# 小書き変換
KOGAKI_MAP = {
    'あ': 'ぁ', 'い': 'ぃ', 'う': 'ぅ', 'え': 'ぇ', 'お': 'ぉ',
    'や': 'ゃ', 'ゆ': 'ゅ', 'よ': 'ょ', 'つ': 'っ', 'わ': 'ゎ',
}

# ひらがな → ローマ字変換 (Karabiner用)
KANA_TO_ROMAJI = {
    'あ': 'a', 'い': 'i', 'う': 'u', 'え': 'e', 'お': 'o',
    'か': 'ka', 'き': 'ki', 'く': 'ku', 'け': 'ke', 'こ': 'ko',
    'さ': 'sa', 'し': 'si', 'す': 'su', 'せ': 'se', 'そ': 'so',
    'た': 'ta', 'ち': 'ti', 'つ': 'tu', 'て': 'te', 'と': 'to',
    'な': 'na', 'に': 'ni', 'ぬ': 'nu', 'ね': 'ne', 'の': 'no',
    'は': 'ha', 'ひ': 'hi', 'ふ': 'hu', 'へ': 'he', 'ほ': 'ho',
    'ま': 'ma', 'み': 'mi', 'む': 'mu', 'め': 'me', 'も': 'mo',
    'や': 'ya', 'ゆ': 'yu', 'よ': 'yo',
    'ら': 'ra', 'り': 'ri', 'る': 'ru', 'れ': 're', 'ろ': 'ro',
    'わ': 'wa', 'を': 'wo', 'ん': 'nn',
    'が': 'ga', 'ぎ': 'gi', 'ぐ': 'gu', 'げ': 'ge', 'ご': 'go',
    'ざ': 'za', 'じ': 'zi', 'ず': 'zu', 'ぜ': 'ze', 'ぞ': 'zo',
    'だ': 'da', 'ぢ': 'di', 'づ': 'du', 'で': 'de', 'ど': 'do',
    'ば': 'ba', 'び': 'bi', 'ぶ': 'bu', 'べ': 'be', 'ぼ': 'bo',
    'ぱ': 'pa', 'ぴ': 'pi', 'ぷ': 'pu', 'ぺ': 'pe', 'ぽ': 'po',
    'ぁ': 'xa', 'ぃ': 'xi', 'ぅ': 'xu', 'ぇ': 'xe', 'ぉ': 'xo',
    'ゃ': 'xya', 'ゅ': 'xyu', 'ょ': 'xyo', 'っ': 'xtu',
    'ー': '-', 'ゔ': 'vu',
}


def load_analyzer_json(filepath: str) -> dict:
    """analyzer JSONを読み込む"""
    with open(filepath, 'r', encoding='utf-8') as f:
        return json.load(f)


def convert_key_to_layout(key: str, use_colemak: bool) -> str:
    """キーをレイアウトに応じて変換"""
    if use_colemak:
        return QWERTY_TO_COLEMAK.get(key, key)
    return key


def generate_hazkey_ansi(data: dict, use_colemak: bool = False) -> str:
    """
    hazkey用ANSIローマ字テーブルを生成

    形式: key[TAB][TAB]output または key[TAB]output
    後置シフト: かな文字 + 修飾キー → 変換後かな
    """
    lines = []
    name = data.get('name', '新月配列')
    layout_type = "Colemak" if use_colemak else "QWERTY"

    conversion = data.get('conversion', {})

    # シフトキーの位置 (QWERTY基準で定義されている)
    shift_star_key = convert_key_to_layout('d', use_colemak)  # ★
    shift_circle_key = convert_key_to_layout('k', use_colemak)  # ☆
    dakuten_key = convert_key_to_layout('l', use_colemak)  # ゛
    handakuten_key = '/'  # ゜ (ANSIでは/キー)

    # ベースレイヤーの文字マッピングを構築
    base_chars = {}  # key -> char
    star_chars = {}  # key -> char (★シフト)
    circle_chars = {}  # key -> char (☆シフト)
    star_dakuten_chars = {}  # key -> char (★+゛)
    circle_dakuten_chars = {}  # key -> char (☆+゛)

    for char, mapping in conversion.items():
        keys = mapping.get('keys', [])
        shift = mapping.get('shift', [])

        if not keys:
            continue

        # 記号はそのまま (・は\キーに固定なのでスキップ)
        if char in ['、', '。', '「', '」', 'ー']:
            key = convert_key_to_layout(keys[0], use_colemak)
            base_chars[key] = char
            continue

        # スキップ
        if char in [' ', '゛', '゜', '！'] or (len(char) == 1 and char.isascii()):
            continue

        # シフト状態を判定
        has_d = 'd' in shift
        has_k = 'k' in shift
        has_l = 'l' in shift

        if len(keys) == 1:
            key = convert_key_to_layout(keys[0], use_colemak)

            if not shift:
                # ベースレイヤー
                base_chars[key] = char
            elif has_d and has_l:
                # ★ + ゛
                star_dakuten_chars[key] = char
            elif has_k and has_l:
                # ☆ + ゛
                circle_dakuten_chars[key] = char
            elif has_d:
                # ★シフト
                star_chars[key] = char
            elif has_k:
                # ☆シフト
                circle_chars[key] = char

    # === 出力開始 ===

    # 左手側ベースレイヤー
    for key in ['q', 'w', 'e', 'r', 't', 'a', 's', 'd', 'f', 'g', 'z', 'x', 'c', 'v', 'b']:
        k = convert_key_to_layout(key, use_colemak)
        if k in base_chars:
            lines.append(f"{k}\t\t{base_chars[k]}")
        elif k == shift_star_key:
            lines.append(f"{k}\t\t★")

    # ☆シフト (左手側)
    for key in ['q', 'w', 'e', 'r', 't', 'a', 's', 'd', 'f', 'g', 'z', 'x', 'c', 'v', 'b']:
        k = convert_key_to_layout(key, use_colemak)
        if k in circle_chars:
            lines.append(f"☆{k}\t{circle_chars[k]}")

    # 右手側ベースレイヤー
    for key in ['y', 'u', 'i', 'o', 'p', '[', 'h', 'j', 'k', 'l', ';', "'", 'n', 'm', ',', '.', '/', '\\']:
        k = convert_key_to_layout(key, use_colemak)
        if k == shift_circle_key:
            lines.append(f"{k}\t\t☆")
        elif k == dakuten_key:
            lines.append(f"{k}\t゛")
        elif key == '/':
            lines.append(f"{k}\t・")
        elif k in base_chars:
            lines.append(f"{k}\t{base_chars[k]}")

    # ★シフト (右手側)
    for key in ['y', 'u', 'i', 'o', 'p', '[', 'h', 'j', 'k', 'l', ';', "'", ']', 'n', 'm', ',', '.', '/']:
        k = convert_key_to_layout(key, use_colemak)
        if k in star_chars:
            lines.append(f"★{k}\t{star_chars[k]}")

    # ★ + ゛キー → わ (゛キーはl(QWERTY)/i(Colemak))
    lines.append(f"★{dakuten_key}\tわ")

    # 濁音 (後置シフト: かな + ゛)
    for base_kana, voiced_kana in DAKUTEN_MAP.items():
        lines.append(f"{base_kana}{dakuten_key}\t{voiced_kana}")


    # 2回押し: 濁音→半濁音 (ば + ゛ = ぱ)
    for voiced, half_voiced in DAKUTEN_TO_HANDAKUTEN_MAP.items():
        lines.append(f"{voiced}{dakuten_key}\t{half_voiced}")

    # 2回押し: 母音→小書き (あ + ゛ = ぁ)
    for vowel, kogaki in VOWEL_TO_KOGAKI_MAP.items():
        lines.append(f"{vowel}{dakuten_key}\t{kogaki}")

    # 2回押し: ゔ→ぅ (小書き)
    for vu, kogaki in VU_TO_KOGAKI_MAP.items():
        lines.append(f"{vu}{dakuten_key}\t{kogaki}")

    # 拗音レイヤー (☆ + ゛ + key)
    if circle_dakuten_chars:
        for key in ['q', 'w', 'e', 'r', 't', 'a', 's', 'd', 'f', 'g', 'z', 'x', 'c', 'v', 'b']:
            k = convert_key_to_layout(key, use_colemak)
            if k in circle_dakuten_chars:
                lines.append(f"☆゛{k}\t{circle_dakuten_chars[k]}")

    # ☆゛ でも みゃ/みゅ/みょ を打てるように (右手キーも追加)
    # ★゛は わ と競合するため、☆゛のみで拗音シフトを実現
    if star_dakuten_chars:
        for key in ['y', 'u', 'i', 'o', 'p', 'h', 'j', 'k', 'l', ';', 'n', 'm']:
            k = convert_key_to_layout(key, use_colemak)
            if k in star_dakuten_chars:
                lines.append(f"☆゛{k}\t{star_dakuten_chars[k]}")

    # シフト2回押し
    lines.append(f"{shift_circle_key}{shift_circle_key}\tも")  # ☆☆ → も
    lines.append(f"{shift_star_key}{shift_star_key}\tら")      # ★★ → ら

    return '\n'.join(lines) + '\n'


def key_to_keycode(key: str) -> str:
    """キーをKarabinerのkey_codeに変換"""
    mapping = {
        ';': 'semicolon',
        "'": 'quote',
        ',': 'comma',
        '.': 'period',
        '/': 'slash',
        '[': 'open_bracket',
        ']': 'close_bracket',
        '-': 'hyphen',
        '=': 'equal_sign',
        '\\': 'backslash',
        'space': 'spacebar',
    }
    return mapping.get(key, key)


def generate_karabiner_json(data: dict, use_colemak: bool = False) -> dict:
    """Karabiner Elements用JSONを生成"""
    name = data.get('name', '新月配列')
    layout_type = "Colemak" if use_colemak else "QWERTY"

    manipulators = []
    conversion = data.get('conversion', {})

    # シフトキー定義
    shift_d = convert_key_to_layout('d', use_colemak)
    shift_k = convert_key_to_layout('k', use_colemak)
    shift_l = convert_key_to_layout('l', use_colemak)

    # ★キー押下 → shift_state = 1
    manipulators.append({
        "type": "basic",
        "from": {"key_code": key_to_keycode(shift_d), "modifiers": {"optional": ["caps_lock"]}},
        "to": [{"set_variable": {"name": "shingetsu_shift", "value": 1}}],
        "conditions": [
            {"type": "variable_if", "name": "shingetsu_shift", "value": 0}
        ]
    })

    # ☆キー押下 → shift_state = 2
    manipulators.append({
        "type": "basic",
        "from": {"key_code": key_to_keycode(shift_k), "modifiers": {"optional": ["caps_lock"]}},
        "to": [{"set_variable": {"name": "shingetsu_shift", "value": 2}}],
        "conditions": [
            {"type": "variable_if", "name": "shingetsu_shift", "value": 0}
        ]
    })

    # ゛キー押下 (シフト状態1or2のとき) → shift_state = 3 or 4
    manipulators.append({
        "type": "basic",
        "from": {"key_code": key_to_keycode(shift_l), "modifiers": {"optional": ["caps_lock"]}},
        "to": [{"set_variable": {"name": "shingetsu_shift", "value": 3}}],
        "conditions": [
            {"type": "variable_if", "name": "shingetsu_shift", "value": 1}
        ]
    })
    manipulators.append({
        "type": "basic",
        "from": {"key_code": key_to_keycode(shift_l), "modifiers": {"optional": ["caps_lock"]}},
        "to": [{"set_variable": {"name": "shingetsu_shift", "value": 4}}],
        "conditions": [
            {"type": "variable_if", "name": "shingetsu_shift", "value": 2}
        ]
    })

    # 各文字のマッピングを生成
    for char, mapping in conversion.items():
        keys = mapping.get('keys', [])
        shift = mapping.get('shift', [])

        if not keys or char in ['゛', '゜']:
            continue
        if len(char) == 1 and char.isascii():
            continue

        # ローマ字出力を取得
        romaji = KANA_TO_ROMAJI.get(char, None)
        if romaji is None:
            continue

        # Colemak変換
        converted_keys = [convert_key_to_layout(k, use_colemak) for k in keys]
        converted_shift = [convert_key_to_layout(s, use_colemak) for s in shift]

        # キー出力シーケンスを作成 (特殊キーはkey_codeに変換)
        def romaji_char_to_keycode(c):
            if c == '-':
                return 'hyphen'
            return c
        to_keys = [{"key_code": romaji_char_to_keycode(c)} for c in romaji]

        shift_d_key = convert_key_to_layout('d', use_colemak)
        shift_k_key = convert_key_to_layout('k', use_colemak)
        shift_l_key = convert_key_to_layout('l', use_colemak)

        # 条件を決定
        if not shift:
            if len(keys) == 1:
                manipulators.append({
                    "type": "basic",
                    "from": {"key_code": key_to_keycode(converted_keys[0]), "modifiers": {"optional": ["caps_lock"]}},
                    "to": to_keys + [
                        {"set_variable": {"name": "shingetsu_shift", "value": 0}}
                    ],
                    "conditions": [
                        {"type": "variable_if", "name": "shingetsu_shift", "value": 0}
                    ]
                })
        else:
            has_d = (shift_d_key in converted_shift)
            has_k = (shift_k_key in converted_shift)
            has_l = (shift_l_key in converted_shift)

            if has_d and has_l:
                shift_state = 3
            elif has_k and has_l:
                shift_state = 4
            elif has_d:
                shift_state = 1
            elif has_k:
                shift_state = 2
            else:
                continue

            if len(keys) == 1:
                manipulators.append({
                    "type": "basic",
                    "from": {"key_code": key_to_keycode(converted_keys[0]), "modifiers": {"optional": ["caps_lock"]}},
                    "to": to_keys + [
                        {"set_variable": {"name": "shingetsu_shift", "value": 0}}
                    ],
                    "conditions": [
                        {"type": "variable_if", "name": "shingetsu_shift", "value": shift_state}
                    ]
                })

    # Escでリセット
    manipulators.append({
        "type": "basic",
        "from": {"key_code": "escape"},
        "to": [
            {"key_code": "escape"},
            {"set_variable": {"name": "shingetsu_shift", "value": 0}}
        ]
    })

    # rulesの中身だけを返す
    return [
        {
            "description": f"{name} - 前置/後置シフト ({layout_type})",
            "manipulators": manipulators
        }
    ]


def main():
    if len(sys.argv) < 2:
        script_dir = Path(__file__).parent.parent
        input_file = script_dir / "results" / "shingetsu_v8.6_analyzer.json"
    else:
        input_file = Path(sys.argv[1])

    if not input_file.exists():
        print(f"Error: {input_file} not found", file=sys.stderr)
        sys.exit(1)

    print(f"読み込み: {input_file}")
    data = load_analyzer_json(str(input_file))

    output_dir = input_file.parent
    base_name = input_file.stem.replace('_analyzer', '')

    # 出力ファイル (5種類のみ)
    outputs = []

    # 1. ANSI QWERTY
    ansi_qwerty = output_dir / f"{base_name}-ansi-qwerty.tsv"
    with open(ansi_qwerty, 'w', encoding='utf-8') as f:
        f.write(generate_hazkey_ansi(data, use_colemak=False))
    outputs.append(ansi_qwerty)

    # 2. ANSI Colemak
    ansi_colemak = output_dir / f"{base_name}-ansi-colemak.tsv"
    with open(ansi_colemak, 'w', encoding='utf-8') as f:
        f.write(generate_hazkey_ansi(data, use_colemak=True))
    outputs.append(ansi_colemak)

    # 3. Karabiner QWERTY
    karabiner_qwerty = output_dir / f"{base_name}-karabiner-qwerty.json"
    with open(karabiner_qwerty, 'w', encoding='utf-8') as f:
        json.dump(generate_karabiner_json(data, use_colemak=False), f, ensure_ascii=False, indent=2)
    outputs.append(karabiner_qwerty)

    # 4. Karabiner Colemak
    karabiner_colemak = output_dir / f"{base_name}-karabiner-colemak.json"
    with open(karabiner_colemak, 'w', encoding='utf-8') as f:
        json.dump(generate_karabiner_json(data, use_colemak=True), f, ensure_ascii=False, indent=2)
    outputs.append(karabiner_colemak)

    print("\n=== 生成ファイル (5種類) ===")
    print(f"1. {input_file.name} (analyzer)")
    for i, f in enumerate(outputs, 2):
        print(f"{i}. {f.name}")

    print("\n完了!")


if __name__ == '__main__':
    main()
