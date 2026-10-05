#!/usr/bin/env python3
"""往模拟器里素笺的 App Group 种记忆数据，给 UI 测试与截图走查用。只写合成数据（人名是设计稿里的虚构角色）。

用法（先把 App 装到模拟器上跑过一次，App Group 容器才存在）：

    UITests/seed/seed.py --device Sujian-PR4              # 现在的格式：三个场景、14 个人、卡片、键盘当前选着小美
    UITests/seed/seed.py --device Sujian-PR4 --legacy     # 老格式：没有 scenes.json，人还挂在写死的 daily/dating/work 上（SceneShots 看迁移）
    UITests/seed/seed.py --device Sujian-PR4 --bad-card   # 另给阿林塞一张关键词只有 1 个字的坏卡（看坏卡不挡场景改名）
    UITests/seed/seed.py --device Sujian-PR4 --clean      # 只清掉 memory/

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

# 场景：id、名字。老格式没有 scenes.json，人直接挂在这三个 id 上
SCENES = [("daily", "日常"), ("dating", "恋爱"), ("work", "工作")]

# 人：序号（id 是序号的 32 位十六进制）、名字、首字母、场景、代号。
# ContactsShots 按 id 点人：第 6 个是小美（卡片最多；带代号「阿美」，键盘上该显示代号），第 1 个是另一个展开对象。
PEOPLE = [
    (1, "阿林", "A", "daily", None),
    (2, "陈老师", "C", "work", None),
    (3, "大鹏", "D", "daily", None),
    (4, "方姐", "F", "work", None),
    (5, "高远", "G", "work", None),
    (6, "小美", "X", "dating", "阿美"),
    (7, "林木", "L", "daily", None),
    (8, "妈妈", "M", "daily", "老妈"),
    (9, "宁宁", "N", "daily", None),
    (10, "秦川", "Q", "work", None),
    (11, "苏苏", "S", "daily", None),
    (12, "唐果", "T", "daily", None),
    (13, "王组长", "W", "work", "组长"),
    (14, "周周", "Z", "daily", None),
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


def seed(root: Path, legacy: bool, bad_card: bool) -> None:
    memory = root / "memory"
    if memory.exists():
        shutil.rmtree(memory)
    memory.mkdir(parents=True)
    now = int(time.time())
    contacts = []
    for n, name, initial, scene, display in PEOPLE:
        contact = {
            "id": hexid(n),
            "name": name,
            "initial": initial,
            "pronoun": "ta_f" if n in (6, 8) else "ta",
            "scene": scene,
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
    if not legacy:
        write(memory / "scenes.json",
              [{"id": sid, "name": sname, "created_at": now - 86400 * 200} for sid, sname in SCENES])
    write(memory / "state.json", {
        "scene": "dating",
        "contact_id": hexid(6),
        "last": {"dating": hexid(6)},
        "used": {hexid(6): now - 3600, hexid(1): now - 86400 * 3},
    })


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--device", default="booted", help="模拟器名字或 UDID，缺省是正开着的那台")
    parser.add_argument("--legacy", action="store_true", help="种成老格式（没有 scenes.json）")
    parser.add_argument("--bad-card", action="store_true", help="另给阿林塞一张不合格的卡")
    parser.add_argument("--clean", action="store_true", help="只清掉 memory/")
    args = parser.parse_args()
    root = container(args.device)
    if args.clean:
        shutil.rmtree(root / "memory", ignore_errors=True)
        print(f"清掉了 {root / 'memory'}")
        return 0
    seed(root, args.legacy, args.bad_card)
    print(f"种好了：{root / 'memory'}（{'老格式' if args.legacy else '现在的格式'}{'，带坏卡' if args.bad_card else ''}）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
