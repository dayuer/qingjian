#!/usr/bin/env python3
"""对照测试的「Mac 拼音」这边：往 TextEdit 发全拼、空格上屏、把文本读回来。

原始输出**只落在 `.lab/mac-compare/` 本机目录**，不入库、不用于训练或蒸馏、不进任何数据文件；
入库的只有每句判定与归类。

输入法**由用户自己在菜单栏切到「拼音 - 简体」**，脚本只读校验、不写任何系统设置：
2026-10-09 用 `defaults write` 改 `AppleSelectedInputSources` 失败过三次，还把用户的输入源列表
短暂改坏（少了一项）——这条路不再走。跑完脚本只提醒用户切回。

用法：python3 tools/mac-pinyin-compare/mac_ime_run.py [--limit N] [--out 文件]
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
SCIM = "com.apple.inputmethod.SCIM.ITABC"


def osa(script: str) -> str:
    result = subprocess.run(["osascript", "-e", script], capture_output=True, text=True, timeout=120)
    if result.returncode != 0:
        raise SystemExit(f"osascript 失败：{script[:60]}… → {result.stderr.strip()}")
    return result.stdout


def current_source() -> str:
    """只读地看当前选中了哪些输入源。"""
    result = subprocess.run(["defaults", "read", HITOOLBOX, "AppleSelectedInputSources"],
                            capture_output=True, text=True)
    return result.stdout


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
    parser.add_argument("--force", action="store_true", help="当前不是拼音 - 简体也照跑（自担风险）")
    args = parser.parse_args()

    rows = [line.split("\t") for line in TASK.read_text(encoding="utf-8").splitlines() if line.strip()]
    if args.limit:
        rows = rows[: args.limit]

    if SCIM not in current_source() and not args.force:
        raise SystemExit("当前输入法不是「拼音 - 简体」：请在菜单栏切换后重跑（脚本不代你切）。")
    print(f"输入法已是拼音 - 简体；跑 {len(rows)} 句", flush=True)

    osa('tell application "TextEdit" to activate')
    time.sleep(1)
    osa('tell application "TextEdit" to make new document')
    time.sleep(1)
    results, extra_spaces = [], 0
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

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text("".join(f"{e}\t{p}\t{g}\t{t}\n" for e, p, g, t in results), encoding="utf-8")
    hit = sum(1 for e, _p, g, _t in results if g == e)
    print(f"写出 {args.out}：{len(results)} 句，Mac 首选命中 {hit}；需要补空格的 {extra_spaces} 次", flush=True)
    print("提醒：跑完了，请把输入法切回原来那个（脚本不代你切）。", flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
