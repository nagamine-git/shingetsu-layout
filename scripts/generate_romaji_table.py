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
    '、': ',', '。': '.', '「': '[', '」': ']',
    # 拗音
    'きゃ': 'kya', 'きゅ': 'kyu', 'きょ': 'kyo',
    'しゃ': 'sya', 'しゅ': 'syu', 'しょ': 'syo',
    'ちゃ': 'tya', 'ちゅ': 'tyu', 'ちょ': 'tyo',
    'にゃ': 'nya', 'にゅ': 'nyu', 'にょ': 'nyo',
    'ひゃ': 'hya', 'ひゅ': 'hyu', 'ひょ': 'hyo',
    'みゃ': 'mya', 'みゅ': 'myu', 'みょ': 'myo',
    'りゃ': 'rya', 'りゅ': 'ryu', 'りょ': 'ryo',
    'ぎゃ': 'gya', 'ぎゅ': 'gyu', 'ぎょ': 'gyo',
    'じゃ': 'zya', 'じゅ': 'zyu', 'じょ': 'zyo',
    'ぢゃ': 'dya', 'ぢゅ': 'dyu', 'ぢょ': 'dyo',
    'びゃ': 'bya', 'びゅ': 'byu', 'びょ': 'byo',
    'ぴゃ': 'pya', 'ぴゅ': 'pyu', 'ぴょ': 'pyo',
    'でゅ': 'dhu',
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
    conversion = data.get('conversion', {})

    # シフトキーの位置 (QWERTY基準で定義されている)
    shift_star_key = convert_key_to_layout('d', use_colemak)  # ★
    shift_circle_key = convert_key_to_layout('k', use_colemak)  # ☆
    dakuten_key = convert_key_to_layout('l', use_colemak)  # ゛

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

        # 記号はそのまま
        if char in ['、', '。', '「', '」']:
            key = convert_key_to_layout(keys[0], use_colemak)
            base_chars[key] = char
            continue

        # スキップ
        if char in [' ', '゛', '゜', '！', 'ー'] or (len(char) == 1 and char.isascii()):
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
    for key in ['y', 'u', 'i', 'o', 'p', '[', 'h', 'j', 'k', 'l', ';', "'", 'n', 'm', ',', '.', '/']:
        k = convert_key_to_layout(key, use_colemak)
        if k == shift_circle_key:
            lines.append(f"{k}\t\t☆")
        elif k == dakuten_key:
            lines.append(f"{k}\t゛")
        elif key == '/':
            lines.append(f"{k}\t・")
        elif k in base_chars:
            lines.append(f"{k}\t{base_chars[k]}")

    # - キー → ー (長音)
    hyphen_key = convert_key_to_layout('-', use_colemak)
    lines.append(f"{hyphen_key}\tー")

    # ★シフト (右手側)
    for key in ['y', 'u', 'i', 'o', 'p', '[', 'h', 'j', 'k', 'l', ';', "'", ']', 'n', 'm', ',', '.', '/']:
        k = convert_key_to_layout(key, use_colemak)
        if k in star_chars:
            lines.append(f"★{k}\t{star_chars[k]}")

    # ★ + ゛キー → わ
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


def romaji_to_keycodes(romaji: str) -> List[dict]:
    """ローマ字文字列をKarabinerのkey_code配列に変換"""
    result = []
    for c in romaji:
        if c == '-':
            result.append({"key_code": "hyphen"})
        elif c == ',':
            result.append({"key_code": "comma"})
        elif c == '.':
            result.append({"key_code": "period"})
        elif c == '[':
            result.append({"key_code": "open_bracket"})
        elif c == ']':
            result.append({"key_code": "close_bracket"})
        else:
            result.append({"key_code": c})
    return result


def generate_karabiner_json(data: dict, use_colemak: bool = False) -> dict:
    """Karabiner Elements用JSONを生成 (月配列2-263形式)"""
    name = data.get('name', '新月配列')
    layout_type = "Colemak" if use_colemak else "QWERTY"

    manipulators = []
    conversion = data.get('conversion', {})

    # シフトキー定義 (QWERTY基準)
    shift_d = convert_key_to_layout('d', use_colemak)  # ★
    shift_k = convert_key_to_layout('k', use_colemak)  # ☆
    shift_l = convert_key_to_layout('l', use_colemak)  # ゛

    # 日本語入力条件
    ja_conditions = [
        {"input_sources": [{"language": "ja"}], "type": "input_source_if"},
        {"input_sources": [{"input_mode_id": "Roman$"}], "type": "input_source_unless"}
    ]

    # last_char値マッピング (濁音用)
    last_char_map = {}
    char_id = 100

    # ベース/シフト文字のマッピングを構築
    base_chars = {}  # key -> char
    star_chars = {}  # key -> char (★シフト)
    circle_chars = {}  # key -> char (☆シフト)

    for char, mapping in conversion.items():
        keys = mapping.get('keys', [])
        shift = mapping.get('shift', [])

        if not keys or len(keys) != 1:
            continue
        if char in ['゛', '゜', ' ', '！']:
            continue
        if len(char) == 1 and char.isascii():
            continue

        key = keys[0]
        has_d = 'd' in shift
        has_k = 'k' in shift
        has_l = 'l' in shift

        if not shift:
            base_chars[key] = char
        elif has_d and not has_l:
            star_chars[key] = char
        elif has_k and not has_l:
            circle_chars[key] = char

    # 各キーのマッピングを生成
    all_keys = ['q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p',
                'a', 's', 'd', 'f', 'g', 'h', 'j', 'k', 'l', ';',
                'z', 'x', 'c', 'v', 'b', 'n', 'm', ',', '.', '/',
                '[', "'", '-']

    for qwerty_key in all_keys:
        key = convert_key_to_layout(qwerty_key, use_colemak)
        keycode = key_to_keycode(key)

        # ★シフトキー (d) - shift_state=1
        if qwerty_key == 'd':
            # ★★ (2回押し): ら を出力
            manipulators.append({
                "type": "basic",
                "conditions": [{"type": "variable_if", "name": "shift_state", "value": 1}] + ja_conditions,
                "from": {"key_code": keycode, "modifiers": {"optional": ["caps_lock"]}},
                "to": romaji_to_keycodes("ra") + [
                    {"set_variable": {"name": "last_char", "value": 0}},
                    {"set_variable": {"name": "shift_state", "value": 0}}
                ]
            })
            # ☆★: ら を出力
            manipulators.append({
                "type": "basic",
                "conditions": [{"type": "variable_if", "name": "shift_state", "value": 2}] + ja_conditions,
                "from": {"key_code": keycode, "modifiers": {"optional": ["caps_lock"]}},
                "to": romaji_to_keycodes("ra") + [
                    {"set_variable": {"name": "last_char", "value": 0}},
                    {"set_variable": {"name": "shift_state", "value": 0}}
                ]
            })
            # 通常時(shift_state=0): シフト状態=1 (★)に設定
            manipulators.append({
                "type": "basic",
                "conditions": [{"type": "variable_if", "name": "shift_state", "value": 0}] + ja_conditions,
                "from": {"key_code": keycode, "modifiers": {"optional": ["caps_lock"]}},
                "to": [
                    {"set_variable": {"name": "last_char", "value": 0}},
                    {"set_variable": {"name": "shift_state", "value": 1}}
                ]
            })
            continue

        # ☆シフトキー (k) - shift_state=2
        if qwerty_key == 'k':
            # ☆☆ (2回押し): も を出力
            manipulators.append({
                "type": "basic",
                "conditions": [{"type": "variable_if", "name": "shift_state", "value": 2}] + ja_conditions,
                "from": {"key_code": keycode, "modifiers": {"optional": ["caps_lock"]}},
                "to": romaji_to_keycodes("mo") + [
                    {"set_variable": {"name": "last_char", "value": 0}},
                    {"set_variable": {"name": "shift_state", "value": 0}}
                ]
            })
            # ★☆: も を出力
            manipulators.append({
                "type": "basic",
                "conditions": [{"type": "variable_if", "name": "shift_state", "value": 1}] + ja_conditions,
                "from": {"key_code": keycode, "modifiers": {"optional": ["caps_lock"]}},
                "to": romaji_to_keycodes("mo") + [
                    {"set_variable": {"name": "last_char", "value": 0}},
                    {"set_variable": {"name": "shift_state", "value": 0}}
                ]
            })
            # 通常時(shift_state=0): シフト状態=2 (☆)に設定
            manipulators.append({
                "type": "basic",
                "conditions": [{"type": "variable_if", "name": "shift_state", "value": 0}] + ja_conditions,
                "from": {"key_code": keycode, "modifiers": {"optional": ["caps_lock"]}},
                "to": [
                    {"set_variable": {"name": "last_char", "value": 0}},
                    {"set_variable": {"name": "shift_state", "value": 2}}
                ]
            })
            continue

        # ゛キー (l)
        if qwerty_key == 'l':
            # ★+゛: 拗音シフト状態(shift_state=4)に移行 (みゃ/みゅ/みょ用)
            manipulators.append({
                "type": "basic",
                "conditions": [{"type": "variable_if", "name": "shift_state", "value": 1}] + ja_conditions,
                "from": {"key_code": keycode, "modifiers": {"optional": ["caps_lock"]}},
                "to": [
                    {"set_variable": {"name": "last_char", "value": 0}},
                    {"set_variable": {"name": "shift_state", "value": 4}}
                ]
            })
            # ☆+゛: 拗音シフト状態(shift_state=3)に移行
            manipulators.append({
                "type": "basic",
                "conditions": [{"type": "variable_if", "name": "shift_state", "value": 2}] + ja_conditions,
                "from": {"key_code": keycode, "modifiers": {"optional": ["caps_lock"]}},
                "to": [
                    {"set_variable": {"name": "last_char", "value": 0}},
                    {"set_variable": {"name": "shift_state", "value": 3}}
                ]
            })
            # ★゛モード(shift_state=4)で゛を押したら: わ を出力
            manipulators.append({
                "type": "basic",
                "conditions": [{"type": "variable_if", "name": "shift_state", "value": 4}] + ja_conditions,
                "from": {"key_code": keycode, "modifiers": {"optional": ["caps_lock"]}},
                "to": romaji_to_keycodes("wa") + [
                    {"set_variable": {"name": "last_char", "value": 0}},
                    {"set_variable": {"name": "shift_state", "value": 0}}
                ]
            })
            # 通常時(shift_state=0): 何も出力しない (後置濁点用のルールは各文字の後に追加)
            manipulators.append({
                "type": "basic",
                "conditions": [{"type": "variable_if", "name": "shift_state", "value": 0}] + ja_conditions,
                "from": {"key_code": keycode, "modifiers": {"optional": ["caps_lock"]}},
                "to": [
                    {"key_code": "vk_none"},
                    {"set_variable": {"name": "last_char", "value": 0}}
                ]
            })
            continue

        # -キー → ー
        if qwerty_key == '-':
            manipulators.append({
                "type": "basic",
                "conditions": ja_conditions,
                "from": {"key_code": keycode, "modifiers": {"optional": ["caps_lock"]}},
                "to": [{"key_code": "hyphen"}, {"set_variable": {"name": "last_char", "value": 0}}]
            })
            continue

        # ★シフト面の文字
        star_char = star_chars.get(qwerty_key)
        if star_char:
            romaji = KANA_TO_ROMAJI.get(star_char)
            if romaji:
                char_id += 1
                last_char_map[star_char] = char_id
                dakuten_keycode = key_to_keycode(convert_key_to_layout('l', use_colemak))

                manipulators.append({
                    "type": "basic",
                    "conditions": [{"type": "variable_if", "name": "shift_state", "value": 1}] + ja_conditions,
                    "from": {"key_code": keycode, "modifiers": {"optional": ["caps_lock"]}},
                    "to": romaji_to_keycodes(romaji) + [
                        {"set_variable": {"name": "last_char", "value": char_id}},
                        {"set_variable": {"name": "shift_state", "value": 0}}
                    ]
                })

                # ★シフト文字の濁音変換
                if star_char in DAKUTEN_MAP:
                    voiced = DAKUTEN_MAP[star_char]
                    voiced_romaji = KANA_TO_ROMAJI.get(voiced)
                    if voiced_romaji:
                        voiced_char_id = char_id + 500
                        manipulators.append({
                            "type": "basic",
                            "conditions": [{"type": "variable_if", "name": "last_char", "value": char_id}] + ja_conditions,
                            "from": {"key_code": dakuten_keycode, "modifiers": {"optional": ["caps_lock"]}},
                            "to": [{"key_code": "delete_or_backspace"}] + romaji_to_keycodes(voiced_romaji) + [
                                {"set_variable": {"name": "last_char", "value": voiced_char_id}}
                            ]
                        })
                        if voiced in DAKUTEN_TO_HANDAKUTEN_MAP:
                            handakuten = DAKUTEN_TO_HANDAKUTEN_MAP[voiced]
                            handakuten_romaji = KANA_TO_ROMAJI.get(handakuten)
                            if handakuten_romaji:
                                manipulators.append({
                                    "type": "basic",
                                    "conditions": [{"type": "variable_if", "name": "last_char", "value": voiced_char_id}] + ja_conditions,
                                    "from": {"key_code": dakuten_keycode, "modifiers": {"optional": ["caps_lock"]}},
                                    "to": [{"key_code": "delete_or_backspace"}, {"key_code": "delete_or_backspace"}] + romaji_to_keycodes(handakuten_romaji) + [
                                        {"set_variable": {"name": "last_char", "value": 0}}
                                    ]
                                })

                # ★シフト文字の小書き変換
                if star_char in VOWEL_TO_KOGAKI_MAP:
                    kogaki = VOWEL_TO_KOGAKI_MAP[star_char]
                    kogaki_romaji = KANA_TO_ROMAJI.get(kogaki)
                    if kogaki_romaji:
                        manipulators.append({
                            "type": "basic",
                            "conditions": [{"type": "variable_if", "name": "last_char", "value": char_id}] + ja_conditions,
                            "from": {"key_code": dakuten_keycode, "modifiers": {"optional": ["caps_lock"]}},
                            "to": [{"key_code": "delete_or_backspace"}] + romaji_to_keycodes(kogaki_romaji) + [
                                {"set_variable": {"name": "last_char", "value": 0}}
                            ]
                        })

        # ☆シフト面の文字
        circle_char = circle_chars.get(qwerty_key)
        if circle_char:
            romaji = KANA_TO_ROMAJI.get(circle_char)
            if romaji:
                char_id += 1
                last_char_map[circle_char] = char_id
                dakuten_keycode = key_to_keycode(convert_key_to_layout('l', use_colemak))

                manipulators.append({
                    "type": "basic",
                    "conditions": [{"type": "variable_if", "name": "shift_state", "value": 2}] + ja_conditions,
                    "from": {"key_code": keycode, "modifiers": {"optional": ["caps_lock"]}},
                    "to": romaji_to_keycodes(romaji) + [
                        {"set_variable": {"name": "last_char", "value": char_id}},
                        {"set_variable": {"name": "shift_state", "value": 0}}
                    ]
                })

                # ☆シフト文字の濁音変換
                if circle_char in DAKUTEN_MAP:
                    voiced = DAKUTEN_MAP[circle_char]
                    voiced_romaji = KANA_TO_ROMAJI.get(voiced)
                    if voiced_romaji:
                        voiced_char_id = char_id + 500
                        manipulators.append({
                            "type": "basic",
                            "conditions": [{"type": "variable_if", "name": "last_char", "value": char_id}] + ja_conditions,
                            "from": {"key_code": dakuten_keycode, "modifiers": {"optional": ["caps_lock"]}},
                            "to": [{"key_code": "delete_or_backspace"}] + romaji_to_keycodes(voiced_romaji) + [
                                {"set_variable": {"name": "last_char", "value": voiced_char_id}}
                            ]
                        })
                        if voiced in DAKUTEN_TO_HANDAKUTEN_MAP:
                            handakuten = DAKUTEN_TO_HANDAKUTEN_MAP[voiced]
                            handakuten_romaji = KANA_TO_ROMAJI.get(handakuten)
                            if handakuten_romaji:
                                manipulators.append({
                                    "type": "basic",
                                    "conditions": [{"type": "variable_if", "name": "last_char", "value": voiced_char_id}] + ja_conditions,
                                    "from": {"key_code": dakuten_keycode, "modifiers": {"optional": ["caps_lock"]}},
                                    "to": [{"key_code": "delete_or_backspace"}, {"key_code": "delete_or_backspace"}] + romaji_to_keycodes(handakuten_romaji) + [
                                        {"set_variable": {"name": "last_char", "value": 0}}
                                    ]
                                })

                # ☆シフト文字の小書き変換
                if circle_char in VOWEL_TO_KOGAKI_MAP:
                    kogaki = VOWEL_TO_KOGAKI_MAP[circle_char]
                    kogaki_romaji = KANA_TO_ROMAJI.get(kogaki)
                    if kogaki_romaji:
                        manipulators.append({
                            "type": "basic",
                            "conditions": [{"type": "variable_if", "name": "last_char", "value": char_id}] + ja_conditions,
                            "from": {"key_code": dakuten_keycode, "modifiers": {"optional": ["caps_lock"]}},
                            "to": [{"key_code": "delete_or_backspace"}] + romaji_to_keycodes(kogaki_romaji) + [
                                {"set_variable": {"name": "last_char", "value": 0}}
                            ]
                        })

        # ベース面の文字 (shift_state=0のときのみ)
        base_char = base_chars.get(qwerty_key)
        if base_char:
            romaji = KANA_TO_ROMAJI.get(base_char)
            if romaji:
                char_id += 1
                last_char_map[base_char] = char_id
                dakuten_keycode = key_to_keycode(convert_key_to_layout('l', use_colemak))

                manipulators.append({
                    "type": "basic",
                    "conditions": [{"type": "variable_if", "name": "shift_state", "value": 0}] + ja_conditions,
                    "from": {"key_code": keycode, "modifiers": {"optional": ["caps_lock"]}},
                    "to": romaji_to_keycodes(romaji) + [
                        {"set_variable": {"name": "last_char", "value": char_id}}
                    ]
                })

                # 濁音変換 (か+゛→が)
                if base_char in DAKUTEN_MAP:
                    voiced = DAKUTEN_MAP[base_char]
                    voiced_romaji = KANA_TO_ROMAJI.get(voiced)
                    if voiced_romaji:
                        # 濁音用のlast_char値を設定
                        voiced_char_id = char_id + 500  # 濁音用のID
                        manipulators.append({
                            "type": "basic",
                            "conditions": [{"type": "variable_if", "name": "last_char", "value": char_id}] + ja_conditions,
                            "from": {"key_code": dakuten_keycode, "modifiers": {"optional": ["caps_lock"]}},
                            "to": [{"key_code": "delete_or_backspace"}] + romaji_to_keycodes(voiced_romaji) + [
                                {"set_variable": {"name": "last_char", "value": voiced_char_id}}
                            ]
                        })

                        # 半濁音変換 (ば+゛→ぱ)
                        if voiced in DAKUTEN_TO_HANDAKUTEN_MAP:
                            handakuten = DAKUTEN_TO_HANDAKUTEN_MAP[voiced]
                            handakuten_romaji = KANA_TO_ROMAJI.get(handakuten)
                            if handakuten_romaji:
                                manipulators.append({
                                    "type": "basic",
                                    "conditions": [{"type": "variable_if", "name": "last_char", "value": voiced_char_id}] + ja_conditions,
                                    "from": {"key_code": dakuten_keycode, "modifiers": {"optional": ["caps_lock"]}},
                                    "to": [{"key_code": "delete_or_backspace"}, {"key_code": "delete_or_backspace"}] + romaji_to_keycodes(handakuten_romaji) + [
                                        {"set_variable": {"name": "last_char", "value": 0}}
                                    ]
                                })

                        # ゔ→ぅ変換
                        if voiced in VU_TO_KOGAKI_MAP:
                            kogaki = VU_TO_KOGAKI_MAP[voiced]
                            kogaki_romaji = KANA_TO_ROMAJI.get(kogaki)
                            if kogaki_romaji:
                                manipulators.append({
                                    "type": "basic",
                                    "conditions": [{"type": "variable_if", "name": "last_char", "value": voiced_char_id}] + ja_conditions,
                                    "from": {"key_code": dakuten_keycode, "modifiers": {"optional": ["caps_lock"]}},
                                    "to": [{"key_code": "delete_or_backspace"}, {"key_code": "delete_or_backspace"}] + romaji_to_keycodes(kogaki_romaji) + [
                                        {"set_variable": {"name": "last_char", "value": 0}}
                                    ]
                                })

                # 母音→小書き (あ+゛→ぁ)
                if base_char in VOWEL_TO_KOGAKI_MAP:
                    kogaki = VOWEL_TO_KOGAKI_MAP[base_char]
                    kogaki_romaji = KANA_TO_ROMAJI.get(kogaki)
                    if kogaki_romaji:
                        manipulators.append({
                            "type": "basic",
                            "conditions": [{"type": "variable_if", "name": "last_char", "value": char_id}] + ja_conditions,
                            "from": {"key_code": dakuten_keycode, "modifiers": {"optional": ["caps_lock"]}},
                            "to": [{"key_code": "delete_or_backspace"}] + romaji_to_keycodes(kogaki_romaji) + [
                                {"set_variable": {"name": "last_char", "value": 0}}
                            ]
                        })

    # 拗音シフト (☆+゛+key) - ぴゃ, ぴゅ, ぴょ, じゃ, etc. (shift_state=3)
    circle_dakuten_chars = {}
    for char, mapping in conversion.items():
        keys = mapping.get('keys', [])
        shift = mapping.get('shift', [])
        if len(keys) == 1 and 'k' in shift and 'l' in shift:
            circle_dakuten_chars[keys[0]] = char

    for qwerty_key, char in circle_dakuten_chars.items():
        romaji = KANA_TO_ROMAJI.get(char)
        if romaji:
            key = convert_key_to_layout(qwerty_key, use_colemak)
            keycode = key_to_keycode(key)
            manipulators.append({
                "type": "basic",
                "conditions": [{"type": "variable_if", "name": "shift_state", "value": 3}] + ja_conditions,
                "from": {"key_code": keycode, "modifiers": {"optional": ["caps_lock"]}},
                "to": romaji_to_keycodes(romaji) + [
                    {"set_variable": {"name": "last_char", "value": 0}},
                    {"set_variable": {"name": "shift_state", "value": 0}}
                ]
            })

    # 拗音シフト (★+゛+key) - みゃ, みゅ, みょ (shift_state=4)
    star_dakuten_chars = {}
    for char, mapping in conversion.items():
        keys = mapping.get('keys', [])
        shift = mapping.get('shift', [])
        if len(keys) == 1 and 'd' in shift and 'l' in shift:
            star_dakuten_chars[keys[0]] = char

    for qwerty_key, char in star_dakuten_chars.items():
        romaji = KANA_TO_ROMAJI.get(char)
        if romaji:
            key = convert_key_to_layout(qwerty_key, use_colemak)
            keycode = key_to_keycode(key)
            manipulators.append({
                "type": "basic",
                "conditions": [{"type": "variable_if", "name": "shift_state", "value": 4}] + ja_conditions,
                "from": {"key_code": keycode, "modifiers": {"optional": ["caps_lock"]}},
                "to": romaji_to_keycodes(romaji) + [
                    {"set_variable": {"name": "last_char", "value": 0}},
                    {"set_variable": {"name": "shift_state", "value": 0}}
                ]
            })

    # rules[の中身だけを返す (object)
    return {
        "description": f"{name} - 前置/後置シフト ({layout_type})",
        "manipulators": manipulators
    }


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
