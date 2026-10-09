#!/usr/bin/env python3
"""对照测试的「Mac 拼音」这边：往 TextEdit 发全拼、空格上屏、把文本读回来。

原始输出**只落在 `.lab/mac-compare/` 本机目录**，不入库、不用于训练或蒸馏、不进任何数据文件；
入库的只有每句判定与归类。

跑之前会先切到「拼音 - 简体」，跑完把输入法恢复原样（原值先存下来）。
发按键要占用用户的键盘与输入法，别在用户用电脑时跑。

用法：python3 tools/mac-pinyin-compare/mac_ime_run.py [--limit N] [--out 文件] [--dry-run]
      --limit N  只跑前 N 句（先小跑验证用）
"""

import argparse
import pathlib
import subprocess
import sys
import time

TASK = pathlib.Path(__file__).resolve().parent / "task-300.tsv"
OUT_DIR = pathlib.Path("/Users/liyuqing/sproot/qingjian-neural/.lab/mac-compare")
HITOOLBOX = "com.apple.HIToolbox"
PYINYIN_SOURCE = (
    '{InputSourceKind = "Input Mode"; "Input Mode" = "com.apple.inputmethod.SCIM.ITABC"; '
    '"Bundle ID" = "com.apple.inputmethod.SCIM"; }'
)


def osa(script: str) -> str:
    result = subprocess.run(["osascript", "-e", script], capture_output=True, text=True, timeout=120)
    if result.returncode != 0:
        raise SystemExit(f"osascript 失败：{script[:60]}… → {result.stderr.strip()}")
    return result.stdout


def current_source() -> str:
    result = subprocess.run(["defaults", "read", HITOOLBOX, "AppleSelectedInputSources"],
                            capture_output=True, text=True)
    return result.stdout.strip()


def switch_source(value: str) -> None:
    subprocess.run(["defaults", "write", HITOOLBOX, "AppleSelectedInputSources", value], check=True)
    subprocess.run(["killall", "TextInputMenuAgent"], capture_output=True)


def read_document() -> str:
    return osa('tell application "TextEdit" to get text of document 1')


def clear_document() -> None:
    osa('tell application "System Events" to keystroke "a" using command down')
    osa('tell application "System Events" to key code 51')


def type_pinyin(pinyin: str) -> None:
    osa(f'tell application "System Events" to keystroke "{pinyin}"')


def press_space() -> None:
    osa('tell application "System Events" to key code 49')


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--limit", type=int, default=None)
    parser.add_argument("--out", type=pathlib.Path, default=OUT_DIR / "mac-raw.tsv")
    args = parser.parse_args()

    rows = [line.split("\t") for line in TASK.read_text(encoding="utf-8").splitlines() if line.strip()]
    if args.limit:
        rows = rows[: args.limit]

    original = current_source()
    print(f"原输入法设置已存（{len(original)} 字节）", flush=True)
    if not args.dry_run:
        switch_source(PYINYIN_SOURCE)
        time.sleep(4)
        after = current_source()
        if "SCIM" not in after:
            switch_source(original)
            raise SystemExit(f"切到拼音 - 简体没成功，当前是：{after[:200]}")

    osa('tell application "TextEdit" to activate')
    time.sleep(1)
    osa('tell application "TextEdit" to make new document')
    time.sleep(1)
    results, extra_spaces = [], 0
    try:
        for index, (expected, pinyin, *_rest) in enumerate(rows, 1):
            clear_document()
            type_pinyin(pinyin)
            press_space()
            got = read_document().strip()
            tries = 0
            # 一整串拼音有时被切成几段，一次空格只上屏前一段：比目标短就再按一次（最多 3 次）
            while len(got) < len(expected) and tries < 2:
                press_space()
                got = read_document().strip()
                tries += 1
                extra_spaces += 1
            results.append((expected, pinyin, got, tries))
            if index % 25 == 0 or index == len(rows):
                print(f"  {index}/{len(rows)}", flush=True)
            time.sleep(0.4)
    finally:
        if not args.dry_run:
            switch_source(original)
            subprocess.run(["killall", "TextInputMenuAgent"], capture_output=True)
            print("输入法已恢复原设置", flush=True)

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text("".join(f"{e}\t{p}\t{g}\t{t}\n" for e, p, g, t in results), encoding="utf-8")
    hit = sum(1 for e, _p, g, _t in results if g == e)
    print(f"写出 {args.out}：{len(results)} 句，Mac 首选命中 {hit}；需要补空格的 {extra_spaces} 次", flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
