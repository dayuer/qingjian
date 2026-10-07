#!/usr/bin/env python3
"""比对两处同意说明的文案是否一字一致。

iOS 的 `ConsentSheet.ConsentCopy` 是唯一来源，Mac 的 `cloud_prompt.consent_points` 是副本
（一端 Swift、一端 Rust，没法共享一份代码，见 cloud/docs/design.md 的「同一份文案在两端的副本」）。
这个脚本按字符串逐个比，改了一边忘了另一边时会报出来。

用法：python3 cloud/scripts/check-consent-copy.py
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
IOS = ROOT / "cloud/ios/App/Account/ConsentSheet.swift"
MAC = ROOT / "apps/macos/src/menubar/cloud_prompt.rs"

# Swift 里一句一段的写法：取出 case 后面那对括号里的标题与三条要点
IOS_CASE = re.compile(r'case \.(\w+):\s*\(\s*"([^"]+)",\s*\[(.*?)\]\s*\)', re.S)
# Rust 里同一份：[ "…", "…", "…" ]
MAC_ARM = re.compile(r'"(\w+)" => Some\(\s*\(\s*"([^"]+)",\s*\[(.*?)\]\s*,?\s*\)\s*\)', re.S)
STRING = re.compile(r'"((?:[^"\\]|\\.)*)"')


def texts(block: str) -> list[str]:
    return [s.replace('\\"', '"') for s in STRING.findall(block)]


def swift_copy() -> dict[str, tuple[str, list[str]]]:
    source = IOS.read_text(encoding="utf-8")
    out = {}
    for name, title, block in IOS_CASE.findall(source):
        out[name] = (title, texts(block))
    return out


def rust_copy() -> dict[str, tuple[str, list[str]]]:
    source = MAC.read_text(encoding="utf-8")
    out = {}
    for name, title, block in MAC_ARM.findall(source):
        out[name] = (title, texts(block))
    return out


def main() -> int:
    for path in (IOS, MAC):
        if not path.exists():
            print(f"找不到 {path}", file=sys.stderr)
            return 2
    ios = swift_copy()
    mac = rust_copy()
    if not ios or not mac:
        print("有一边一份文案都没解析出来，检查脚本里的正则", file=sys.stderr)
        return 2

    # Swift 那边功能名是 memory / inputLog，Rust 是 memory / input_log：先归一化再比
    def norm(name: str) -> str:
        return {"inputLog": "input_log"}.get(name, name)

    ios = {norm(k): v for k, v in ios.items()}
    mac = {norm(k): v for k, v in mac.items()}

    failed = False
    for name in sorted(set(ios) | set(mac)):
        a, b = ios.get(name), mac.get(name)
        if a is None or b is None:
            print(f"✗ {name}：只有一边有（iOS {'有' if a else '没有'}，Mac {'有' if b else '没有'}）")
            failed = True
            continue
        if a[0] != b[0]:
            print(f"✗ {name} 标题不同：\n  iOS: {a[0]}\n  Mac: {b[0]}")
            failed = True
        if a[1] != b[1]:
            failed = True
            print(f"✗ {name} 要点不同（共 {len(a[1])} 对 {len(b[1])} 条）：")
            for i in range(max(len(a[1]), len(b[1]))):
                left = a[1][i] if i < len(a[1]) else "（缺）"
                right = b[1][i] if i < len(b[1]) else "（缺）"
                if left != right:
                    print(f"  [{i}] iOS: {left}\n      Mac: {right}")
        else:
            print(f"✓ {name}（{len(a[1])} 条）一致")

    if failed:
        print("\n两处文案不一致，改一处要改另一处（cloud/docs/design.md）。")
        return 1
    print("\n全部一致。")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
