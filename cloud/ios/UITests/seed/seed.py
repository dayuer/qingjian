#!/usr/bin/env python3
"""往模拟器里素笺的 App Group 种记忆数据，给 UI 测试与截图走查用。只写合成数据（人名是设计稿里的虚构角色）。

用法（先把 App 装到模拟器上跑过一次，App Group 容器才存在）：

    UITests/seed/seed.py --device Sujian-PR4              # 现在的格式：14 个人、卡片、键盘当前选着小美
    UITests/seed/seed.py --device Sujian-PR4 --legacy     # 老格式：卡片文件是光秃秃的数组（没有 {"rev","cards"}）
    UITests/seed/seed.py --device Sujian-PR4 --bad-card   # 另给阿林塞一张关键词只有 1 个字的坏卡
    UITests/seed/seed.py --device Sujian-PR4 --recording  # 另写一份 logs = true 的 cloud.toml（键盘「记录中」标记用）
    UITests/seed/seed.py --device Sujian-PR4 --clean      # 清掉 memory/、cloud.toml 与暂停文件

每次都先清掉整个 memory/ 再写。容器路径用 `xcrun simctl get_app_container <设备> sujian.synon.ai group.sujian.synon.ai` 找。
"""

import argparse
import json
import shutil
import subprocess
import sys
import time
from datetime import datetime, timedelta, timezone
from pathlib import Path

GROUP = "group.sujian.synon.ai"
BUNDLE = "sujian.synon.ai"

# 人：序号（id 是序号的 32 位十六进制）、名字、首字母、代号。
# ContactsShots 按 id 点人：第 6 个是小美（卡片最多；带代号「阿美」，键盘上该显示代号），第 1 个是另一个展开对象。
PEOPLE = [
    (1, "阿林", "A", None),
    (2, "陈老师", "C", None),
    (3, "大鹏", "D", None),
    (4, "方姐", "F", None),
    (5, "高远", "G", None),
    (6, "小美", "X", "阿美"),
    (7, "林木", "L", None),
    (8, "妈妈", "M", "老妈"),
    (9, "宁宁", "N", None),
    (10, "秦川", "Q", None),
    (11, "苏苏", "S", None),
    (12, "唐果", "T", None),
    (13, "王组长", "W", "组长"),
    (14, "周周", "Z", None),
]

TZ = timezone(timedelta(hours=8))


def hexid(n: int) -> str:
    return f"{n:032x}"


def day(offset: int) -> str:
    return (datetime.now(TZ).date() + timedelta(days=offset)).isoformat()


def cards_for(n: int, now: int) -> list:
    """每个人几张卡；小美的最多，覆盖提示（考试、科目二）、日子、约定、偏好（香菜，对上键盘冲突屏的样例）。"""
    base = 1000 * n

    def card(k, kind, text, keywords=(), when=None):
        return {
            "id": hexid(base + k),
            "kind": kind,
            "text": text,
            "keywords": list(keywords),
            "when": when,
            "source": "manual",
            "confirmed": True,
            "faded": False,
            "seq": 0,
            "updated_at": 0,
            "created_at": now - 86400 * (30 - k),
            "touched_at": now - 86400 * (30 - k),
        }

    if n == 6:
        return [
            card(1, "recent", "周三考科目二，今天该出成绩了", ["考试", "科目二"]),
            card(2, "date", "生日", (), day(1)),
            card(3, "promise", "答应周末一起去看电影", ["电影"], day(5)),
            card(4, "preference", "不吃香菜", ["香菜"]),
            card(5, "preference", "喜欢喝无糖乌龙", ["乌龙"]),
            card(6, "other", "养了一只橘猫，叫馒头", ["橘猫", "馒头"]),
            card(7, "date", "认识的纪念日", (), "2024-11-02"),
            card(8, "recent", "最近在学游泳", ["游泳"]),
        ]
    if n == 1:
        return [
            card(1, "promise", "下周还他那本书", ["那本书"], day(3)),
            card(2, "preference", "跑步只在早上", ["跑步"]),
            card(3, "recent", "刚换了新工作", ["新工作"]),
        ]
    if n == 8:
        return [
            card(1, "date", "生日", (), day(12)),
            card(2, "preference", "血压高，少吃咸的", ["咸的"]),
        ]
    return [card(1, "other", f"第 {n} 个人的一条样例记忆", ["样例"])]


def container(device: str) -> Path:
    out = subprocess.run(
        ["xcrun", "simctl", "get_app_container", device, BUNDLE, GROUP],
        check=True, capture_output=True, text=True,
    ).stdout.strip()
    return Path(out) / "Library/Application Support/Qingjian"


def write(path: Path, value) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2), encoding="utf-8")


def seed_recording(root: Path) -> None:
    """给键盘走查造「登录了、开了上传输入日志」的样子：标记只看配置，服务器连不上不影响。
    真的登录要建空间、拿令牌，走查里不划算。

    令牌必须以 `sjt_` 开头：桥的 `CloudConfig::load` 认这个前缀才算登录，否则整份配置当没配置
    （踩过：写成 `seed-recording` 时标记一直不出）。

    同时清掉暂停文件：上一次走查点到「一直暂停」的话，这一次一上来就是「已暂停」，
    连着的深色那轮会从错的状态开拍（踩过：深色那轮第一下点的是「恢复」而不是「暂停」）。"""
    shutil.rmtree(root / "cloud", ignore_errors=True)
    (root / "cloud.toml").write_text(
        'server = "http://127.0.0.1:9"\ntoken = "sjt_seed-recording"\nlogs = true\n', encoding="utf-8")


def clear_state(root: Path) -> None:
    """回到「没开通云、没在记」的样子：memory/、cloud.toml 与暂停文件都清掉。"""
    shutil.rmtree(root / "memory", ignore_errors=True)
    (root / "cloud.toml").unlink(missing_ok=True)
    shutil.rmtree(root / "cloud", ignore_errors=True)


def seed(root: Path, legacy: bool, bad_card: bool) -> None:
    memory = root / "memory"
    if memory.exists():
        shutil.rmtree(memory)
    memory.mkdir(parents=True)
    now = int(time.time())
    contacts = []
    for n, name, initial, display in PEOPLE:
        contact = {
            "id": hexid(n),
            "name": name,
            "initial": initial,
            "pronoun": "ta_f" if n in (6, 8) else "ta",
            "created_at": now - 86400 * (100 + n),
            "hint_on": True,
            "remind_on": True,
        }
        if display:
            contact["display_name"] = display
        contacts.append(contact)
        cards = cards_for(n, now)
        if bad_card and n == 1:
            bad = dict(cards[0])
            bad["id"] = hexid(1000 * n + 99)
            bad["text"] = "周末想去海边看日落"
            bad["keywords"] = ["海"]
            cards.append(bad)
        # 老格式的卡片文件是光秃秃的数组；现在是 {"rev","cards"}
        write(memory / hexid(n) / "cards.json", cards if legacy else {"rev": 1, "cards": cards})
    write(memory / "contacts.json", contacts)
    write(memory / "state.json", {
        "contact_id": hexid(6),
        "used": {hexid(6): now - 3600, hexid(1): now - 86400 * 3},
    })


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--device", default="booted", help="模拟器名字或 UDID，缺省是正开着的那台")
    parser.add_argument("--legacy", action="store_true", help="种成老格式（卡片文件是光秃秃的数组）")
    parser.add_argument("--bad-card", action="store_true", help="另给阿林塞一张不合格的卡")
    parser.add_argument("--recording", action="store_true", help="另写一份 logs = true 的 cloud.toml")
    parser.add_argument("--clean", action="store_true", help="清掉 memory/、cloud.toml 与暂停文件")
    args = parser.parse_args()
    root = container(args.device)
    if args.clean:
        clear_state(root)
        print(f"清掉了 {root}（memory/、cloud.toml、暂停文件）")
        return 0
    seed(root, args.legacy, args.bad_card)
    if args.recording:
        seed_recording(root)
        print(f"也写了 {root / 'cloud.toml'}（logs = true）")
    print(f"种好了：{root / 'memory'}（{'老格式' if args.legacy else '现在的格式'}{'，带坏卡' if args.bad_card else ''}）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
