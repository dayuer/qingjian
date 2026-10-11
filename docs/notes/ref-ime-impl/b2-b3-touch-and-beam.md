# B2 触摸坐标个性化 / B3 beam 宽调度 —— 复刻规格

取证对象：RefIME 2.2.3 macOS arm64（`/tmp/ref-ime-rea/RefIME.arm64`），纯静态。地址都是 vmaddr。
辅助脚本：`/tmp/b2b3/fn.sh <addr>`（按 addrs.txt 切函数体 objdump）、`/tmp/b2b3/rd.py <addr> <n>`（读常量）、`/tmp/b2b3/stubs.txt`（stub → 导入符号，`powf`/`expf`/`cos`/`exp` 就是这么认出来的）。
所有常数都是 **RefIME 的值，青简要自己重扫**。RefIME 大部分超参来自随包的 capnp 数据文件（`HyperParams` / `PersonalDictParamsProto`），本机没有该文件，凡是从数据文件读的值都写成「数据文件给，值未知」。

---

## B2 触摸坐标个性化

### 1. RefIME 怎么做的

#### 1.1 坐标怎么进引擎

- C 接口 `_wxime_process_input` @0x1006585c4：`(session_id x0, input x1, input_len w2, aux_data x3, aux_len w4)` [推：寄存器落栈顺序 + 日志串 `wxime_process_input [%lu],input len[%u]`]。
- `aux_data` 是 `InputCoordinate` 数组的二进制：`InputCoordinates::InputCoordinates(string_view)` @0x10063398c 要求长度是 16 的倍数，逐 16 字节调 `InputCoordinate(string_view)` @0x1006333d0（断言串 `auxiliary_data.size() == sizeof(InputCoordinate)`）。
- `InputCoordinate`（16 字节）[确：@0x1006333d0 拷贝布局 + `ToJSON` @0x100633674 的键名 `x` / `y` / `timestamp`]：

  ```c
  struct InputCoordinate { uint32_t x; uint32_t y; uint64_t timestamp; };   // x/y 是整数像素/点
  ```
- 进图：`SyllableGraphV3::InputSequenceInsert(string_view key, uint32 pos, optional<InputCoordinate>)` @0x100cf1ffc，每个输入字符带一个 `optional<InputCoordinate>`，存成 `vector<optional<InputCoordinate>>`（元素 24 字节：16 字节值 + engaged@+0x10）；断言串 `input_seq_coordinate.size() == input_seq.size()` [确]。
- 键盘布局：`KeyBoardLayout::KeyBoardLayout(_wxime_keyboard_item* rects, _wxime_keyboard_item* hot, uint32 count, uint32 flags)` @0x1006341a8，由 `JobSetKeyBoardLayout`（session 任务队列里的一种，字符串 `SetKeyBoardLayout`）下发 [确]：

  ```c
  struct KeyRect { uint32_t x_min, x_max, y_min, y_max; };     // _wxime_keyboard_item，16 字节
  struct Item    { uint32_t cx, cy, w, h; uint32_t hot[4]; };   // 32 字节；cx=(x_min+x_max)/2，cy=(y_min+y_max)/2
  struct KeyBoardLayout {
      vector<Item> keys;     // +0x00；count 只认 26（a..z）或 9（T9 的 '1'..'9'）
      uint32_t pitch_x;      // +0x18；26 键 = keys['w'].cx - keys['q'].cx（下标 22-16）；9 键 = keys[1].cx - keys[0].cx
      uint32_t pitch_y;      // +0x1c；26 键 = keys['a'].cy - keys['q'].cy；9 键 = keys[3].cy - keys[0].cy
      uint32_t flags;        // +0x20；(flags & 7) != 0 时不用个人模型、也不学习 [确]，含义未知 [猜：浮动/单手/横屏之类]
      bool     _;            // +0x24
  };
  ```
  **归一化单位是键距（相邻键中心距），不是键宽** [确 @0x100634330..0x1006343ac]。
- 26 键索引是 `c - 'a'`（大写先转小写），9 键索引是 `c - '1'` [确 @0x100967118..0x100967164]。
- `EnvContext+0x4 == 1` 表示 T9（九宫格）[推：同一个标志传给 DynamicCalculator 的 `is_t9`，后者据此选 9 键表]；B3 里很多超参也按它分两套。

#### 1.2 存的是什么：`MemCacheUserCoordinateDict`

- 值 `UserCoordinateDict::ValueItem`（20 字节）[确：`Update` @0x100824f84 的读写偏移 + `LookUpByKey` 的 `smaddl ... #0x14`]：

  ```c
  struct ValueItem { float mean_x, mean_y; float std_x, std_y; uint32_t count; };
  // 初值 {0, 0, 0.5, 0.5, 0}：常量 @0x102e498c0 = [0.0, 0.0, 0.5, 0.5] [确]
  ```
- 键：一个字符（`key[0]` 的字节值，有符号 char 当下标）；`ValidCheck` @0x100825bdc 校验键 [推：长度 1]。
- 存储：**整张表是一个 5120 字节的 blob = 256 槽 × 20 字节**，按字符字节值直接寻址，存在用户画像字典（`UserProfile`，kind=3）的一条记录里 [确 @0x100825e28：`cmp x8, #0x1400`、初始化循环 256 次、`smaddl x0, w9(key[0]), #20, base`]。没有容量上限 / 淘汰（固定 256 槽）。`MemCache` 包在一个底层 `IDict` 上，写经底层持久化（`Impl::Add` 调底层 vtable+0x18 `w1=3` 的 Update(kind=3, lambda)）。
- `LookUpByKey` @0x100825564：总是返回 engaged；底层没有这条画像时返回初值 `{0,0,0.5,0.5,0}` [确]。

#### 1.3 更新：`ValueItem::Update(x, y, alpha, update_std)` @0x100824f84 [确，逐条指令]

```c
void Update(ValueItem* v, float x, float y, float alpha, bool update_std) {
    alpha = fmaxf(fminf(alpha, 0.1f /*0x3dcccccd*/), 1e-4f /*0x38d1b717*/);
    v->mean = (1 - alpha) * v->mean + alpha * (x, y);          // 先更新均值
    if (update_std) {                                          // 用「新」均值算偏差
        d = (x, y) - v->mean;
        v->std = sqrt((1 - alpha) * v->std^2 + alpha * d^2);   // 逐轴，double 算
    }
    v->count += 1;
}
```

#### 1.4 更新时机：`UserCoordinateDictWrapper::onInputEmitted(EmitInputEvent const&)` @0x100966e64

订阅 session 的 `EmitInputEvent`（上屏事件），构造时连到信号 @0x1006ee0f4 [确]。伪代码（偏移是 EmitInputEvent 的字段）：

```c
void onInputEmitted(Wrapper* self, const EmitInputEvent* e) {
    if (!e->learnable /*+0x74*/) return;
    if (e->layout_keys /*vector<Item>@+0x50*/.empty()) return;
    if (e->layout_flags /*+0x70*/ & 7) return;
    if (!e->query_str /*optional<string>@+0x30, engaged@+0x48*/) return;
    LOG("query_str for update coordinate dict : " << *e->query_str);   // 串 @0x102cb942d
    if (query_str.size() == 0 || e->coords /*vector<optional<InputCoordinate>>@+0x90*/.size() != query_str.size()) return;
    for (i = 0; i < query_str.size(); i++) {
        if (!e->coords[i]) continue;
        char c = query_str[i];
        int idx;
        if (keys.size() == 26) { if ('a'<=c<='z') idx=c-'a'; else if ('A'<=c<='Z') {c+=32; idx=c-'a';} else continue; }
        else if (keys.size() == 9) { if ('1'<=c<='9') idx=c-'1'; else continue; }
        else continue;
        float alpha = 0.01f;                                    // 0x3c23d70a [确]
        ValueItem cur = dict->LookUpByKey(&c, 1);
        if (params.dataSize_bits >= 32 && params.f[0] > 0) {    // PersonalDictParamsProto 的 3 个 float，数据文件给
            float lr = params.f[0];                             // 初始学习率
            float floor = 0;
            if (bits >= 64)  lr = params.f[0] - cur.count * params.f[1];  // 按次数线性衰减
            if (bits >= 96)  floor = params.f[2];                         // 下限
            alpha = max(lr, floor);
        }
        float dx = ((float)coord.x - (float)keys[idx].cx) / (float)e->pitch_x /*+0x68*/;
        float dy = ((float)coord.y - (float)keys[idx].cy) / (float)e->pitch_y /*+0x6c*/;
        dict->Add(&c, 1, dx, dy, alpha, /*update_std=*/false);  // [确] w3=0：这条路径只学均值，std 永远停在 0.5
    }
}
```

- 结论：**每个键存一个「用户相对键中心的平均偏移」（EMA），以键距为单位**；方差字段存在但这条路径不更新 [确]；有无别的调用点传 `true` 未查到 [推：没有]。
- 学的是谁的偏移：`query_str[i]` 那个键的中心。`query_str` 是上屏时的哪一串（原始按键还是纠正后的拼写）**未读到构造点** [猜：纠正后的拼写；否则「点在 F/G 交界」学不出偏向 G]。
- 上屏才学、学多少次 = 上屏串的字符数；没有私密判断出现在这里 [推：`learnable@+0x74` 由上层按私密/场景置位，未追]。（订正：D3 取证确认 `EmitInputEvent+0x74` 不是 learnable，是内嵌 `KeyBoardLayout` 的 valid 字段，即「这次上屏带有效键盘布局」；见 [d3-learning-gate.md](d3-learning-gate.md)。）

#### 1.5 打分：`DynamicCalculator::operator()(char typed, char key, KeyBoardLayout const&, InputCoordinate const&)` @0x100b1a620 [确]

字段（构造在 `Searcher::Searcher` @0x100a27904 附近，参数来自 `HyperParams::DynamicCorrectionScoreParams(is_t9)` @0x10073328c 的 6 个 float `p0..p5`，数据文件给）：

```c
struct DynamicCalculator {
    float default_mx /*+0x00*/ = p0, default_my /*+0x04*/ = p1;
    float a    /*+0x08*/ = p4 / (2*PI * p2 * p3 + 1e-8);          // 2π=0x401921fb54442d18，1e-8=0x3e45798ee2308c3a
    float b    /*+0x0c*/ = p5;
    float base /*+0x10*/ = exp(-1.0 / (2 * p2 * p3 + 1e-8));      // libSystem _exp
    IDict* user_coord_dict /*+0x18*/;
    bool  is_t9 /*+0x28*/ = (env->[+4] == 1);
};
float operator()(char typed /*未使用*/, char key, layout, coord) {
    if (is_t9) { if (!('1'<=key<='9') || layout.keys.size()!=9) return 0; idx=key-'1'; }
    else       { if (!('a'<=key<='z') || layout.keys.size()!=26) return 0; idx=key-'a'; }
    float dx = ((float)coord.x - keys[idx].cx) / layout.pitch_x;
    float dy = ((float)coord.y - keys[idx].cy) / layout.pitch_y;
    float mx, my;
    if ((layout.flags & 7) || !user_coord_dict) { mx = default_mx; my = default_my; }
    else { auto v = user_coord_dict->LookUpByKey(&key,1); mx = v ? v.mean_x : 0; my = v ? v.mean_y : 0; }
    float d2 = (dx-mx)^2 + (dy-my)^2;
    return a * powf(base, d2) + b;          // libSystem _powf；= p4/(2πσxσy)·exp(-d2/(2σxσy)) + p5
}
```

- 即**各向同性二维高斯密度**（σ² = p2·p3），中心 = 键中心 + 该键的个人平均偏移，再线性缩放 + 偏置。**个人模型只提供均值，σ 是全局超参**（个人 std 不参与打分）[确]。不是 softmax、不在各键之间归一化 [确]。
- 冷启动：个人均值 0（= 键中心），σ 用超参。

#### 1.6 进候选打分：特征 `dcs` / `cod_score`

- 符号 `features::dynamic_correction_score_abbr` = SSO 串 `"dcs"`（@0x102e95c58），`features::coordinate_score_abbr` = `"cod_score"`（@0x102ea02a0），`features::kb_neighbour_abbr` = `"r"`（@0x102e95840）[确：读常量]。
- 音节特征构建 `SyllableFeatureBuildHelper()::$_0` @0x100b1865c：遍历音节上的敲错操作记录（5 字节一条，`[op, a, b, ...]`，op 0..0x1b 跳表），对 **`r` 类（键盘邻键替换）** 的那条，若该输入位置有坐标、布局是 9/26 键：

  ```c
  dcs += DynamicCalculator(RecordToInputKey(a), RecordToInputKey(b), layout, coords[pos]);   // @0x100b194c8
  ```
  第二个字符 `b` 是被打分的键 [确]；`b` 是「纠正成的键」[推：特征语义 + 学习用纠正串]。同一分支还记计数特征 `r`、`r_<a>_`、`r__<b>` 一类 [确：拼 `0x5f72`="r_" 等]。
- 路径级：`GetDynamicCorrectionFeature` @0x100c455c4 把各音节的 `dcs` 求和，格式化成字符串写进 `cod_score`；`PathV2::PathCoordinateScore` @0x100a48f64 再 `stof` 读回 [确]。用处：`CandidateV2Collector::BuildCandidateV2From`（候选上带坐标分）与 `CandidateV2::SetSentenceSearcherGlobalFeature`（MixRank 全局特征）[确：callers]。音节级 `dcs` 本身进 LR 模型（`PathV2::CalculateScoreByModel` 的特征表）[推]。
- **和 KeyboardNeighbour 增强器的关系**：邻键表是静态的（`utils::kFullKeyboardNeighboursParallel` / `kFullKeyboardNeighboursVertical`，`KeyboardNeighbourAugmenterIteratively`），坐标**不改变生成哪些邻键边**，只在打分时作为 LR 特征给「这条替换边像不像真的」加分 [推：增强器签名里无坐标/布局参数]。

#### 1.7 相关但不在本任务：动态热区

`session::KeyboardHotArea`（监听 `PendingInputDetailInfoEvent`，带 bigram Reader）会发 `WXIME_EVENT_KEYBOARD_HOT_AREA_UPDATED` 给壳，`GetDynamicHotAreaFeature` 记录用 [确：符号/串]。即引擎按上下文预测下一键、让壳放大热区。机制未读，留给后续任务。

### 2. 证据表

| 结论 | 置信度 | 地址 / 字符串 |
|---|---|---|
| InputCoordinate = {u32 x, u32 y, u64 timestamp} | 确 | 0x1006333d0；ToJSON 0x100633674 键名 x/y/timestamp |
| aux_data 按 16 字节切成坐标数组 | 确 | 0x10063398c；`auxiliary_data.size() == sizeof(InputCoordinate)` |
| 布局 Item 由矩形算中心/宽高，归一化用键距 q→w、q→a（T9：1→2、1→4） | 确 | 0x1006341a8..0x1006343ac |
| ValueItem = {mean_x, mean_y, std_x, std_y, count}，初值 {0,0,0.5,0.5,0} | 确 | 0x100824f84；常量 0x102e498c0 |
| 表 = 256×20 字节 blob，按字符字节寻址，UserProfile kind 3 | 确 | 0x100825e28（`#0x1400`、`smaddl #0x14`） |
| EMA 更新，alpha 夹在 [1e-4, 0.1]；std 可选按新均值更新 | 确 | 0x100824f84 |
| 上屏时学，alpha 缺省 0.01，参数版 = max(lr0 − count·decay, floor) | 确 | 0x100966e64（0x3c23d70a；+0xb78/+0xb88） |
| 学习时 update_std = false | 确 | 0x100967244 `mov w3, #0x0` |
| 打分 = p4/(2πσxσy)·exp(−d²/(2σxσy)) + p5，d 相对「键中心 + 个人均值」 | 确 | 0x100b1a620；构造 0x100a27904..0x100a279f8 |
| 打分不归一化、不用个人 std | 确 | 同上 |
| dcs 只在 `r`（邻键替换）操作上算，路径求和成 cod_score | 确 | 0x100b194c8；0x100c455c4；0x100a48f64 |
| query_str 是纠正后的拼写 | 猜 | 未找到 EmitInputEvent 构造点 |
| 邻键边生成不看坐标 | 推 | 增强器为静态表 |
| EnvContext+4==1 即 T9 | 推 | DynamicCalculator `is_t9` 来源 0x100a279c0 |

### 3. 青简落点

现状（已读代码）：

- iOS 键区触摸：`cloud/ios/Keyboard/Sources/KeyTouchView.swift`（`touchesBegan` 里按最近格子定键，记 `origins[touch]` 但不往外传；`onPress(index)` / `onRelease(index, cancelled)`）→ `KeyboardViewController.mountTouchView()` → `KeyboardModel.release(slot:key:cancelled:)` → `tap` → `typeLetter(_:)` → `Engine.swift` 的 `push(_:)` → C 桥 `qj_push(session, c)`（`cloud/crates/qingjian-cloud-bridge/src/lib.rs:96`）→ `session/mod.rs:214 Session::push` → `Engine::push`（`crates/qingjian-core/src/engine/composing.rs:171`）→ `Composition::push`（`crates/qingjian-core/src/composition.rs:54`，已有与字符一一对应的 `shifted: Vec<bool>`，正好是加触点的样板）。
- 格子几何：`cloud/ios/Keyboard/Sources/KeyboardLayout.swift` `slots(layer:showsGlobe:size:) -> [KeySlot]`，`KeySlot.cell` 是触摸范围矩形（键区坐标）。
- 敲错边：`crates/qingjian-core/src/engine/query/lookup.rs` `expand_positions`：每个完整音节 `typo::variants(pattern.text)` → `expanded.push_alternative(index, text, self.typo_costs.typo_cost(kind, accepted))`；邻键判定 `correction/typo/mod.rs::adjacent`（静态 QWERTY 三排、下排右错半键）；代价 `correction/costs.rs::TypoCosts`（substitute 5.0）。
- 上屏学习：`engine/commit/mod.rs` `commit_with` 里 `accepted_typos(candidate)` 得到 `(typed 音节, intended 音节)`，`self.learner.record_typo(...)`；私密由 `engine/privacy.rs` 合成 `self.private` 并 `learner.set_private`。
- 落盘：`qingjian-learning` 的 `FrequencyLearner`（`user-typos.tsv` 同目录）。

与 RefIME 的取舍（按青简约束改）：

| RefIME | 青简 |
|---|---|
| 引擎收原始像素 + 布局，自己归一化 | 壳归一化（只有壳知道几何）：传「相对所按键中心的偏移 / 键距」。Core 还需邻键之间的相对几何：Core 内置 iOS QWERTY 交错（行偏移 0 / 0.5 / 1.5 键距），壳可用 `set_key_geometry` 覆盖。Core 不碰坐标系、不碰像素 |
| 坐标分作为 LR 特征 | 青简无 LR：直接改**邻键替换边的代价**（`TypoKind::Substitute`），无触点时代价与现在逐位相同 |
| 只学均值、σ 全局 | 同：v1 学均值，σ 全局常数；方差字段先存不用（与 RefIME 一致，给以后留口） |
| 256 槽 blob | 26 行 TSV，跟 `user-typos.tsv` 同目录、同保存节奏 |
| 上屏学习 | 同；私密输入不学（`self.private`），不进分区、不上传 |

新增 / 改动（类型一个文件，签名如下）：

1. `crates/qingjian-core/src/touch/mod.rs`（新模块，只做 mod 声明与 re-export）
   - `touch/sample.rs`：
     ```rust
     /// 一次按键的触点：相对所按键中心的偏移，以键距为单位（右、下为正）。
     #[derive(Debug, Clone, Copy, PartialEq)]
     pub struct TouchSample { pub dx: f32, pub dy: f32 }
     ```
   - `touch/geometry.rs`：
     ```rust
     /// 各字母键中心，以键距为单位；缺省是 iOS 全键盘（第二排右移 0.5、第三排右移 1.5）。
     pub struct KeyGeometry { centers: [(f32, f32); 26] }
     impl KeyGeometry {
         pub fn ios_qwerty() -> Self;
         pub fn from_centers(centers: &[(char, f32, f32)]) -> Option<Self>; // 缺字母返回 None
         pub fn offset(&self, from: char, to: char) -> Option<(f32, f32)>;   // to 中心 − from 中心
     }
     ```
   - `touch/key_stats.rs`：
     ```rust
     #[derive(Debug, Clone, Copy, Default)]
     pub struct KeyStats { pub mean_x: f32, pub mean_y: f32, pub var_x: f32, pub var_y: f32, pub count: u32 }
     impl KeyStats { pub fn update(&mut self, dx: f32, dy: f32, alpha: f32); } // RefIME Update 的公式，var 一并更新
     ```
   - `touch/model.rs`：
     ```rust
     pub struct TouchModel { keys: [KeyStats; 26], params: TouchParams, dirty: bool }
     impl TouchModel {
         pub fn learn(&mut self, intended: char, touch_rel_intended: (f32, f32));     // EMA，alpha 见 params
         /// 触点（相对 typed 中心）落在 key 上的对数似然（不含常数项）：−‖t − (c_key − c_typed + m_key)‖² / (2σ²)
         pub fn log_likelihood(&self, geometry: &KeyGeometry, typed: char, touch: TouchSample, key: char) -> f32;
         /// 邻键替换边要加的代价（nat），见下式；typed/intended 不相邻或没有触点时返回 0
         pub fn substitute_adjustment(&self, geometry: &KeyGeometry, typed: char, intended: char, touch: TouchSample) -> f64;
         pub fn to_tsv(&self) -> String; pub fn from_tsv(s: &str) -> (Self, usize /*跳过行数*/);
     }
     ```
   - `touch/params.rs`：`pub struct TouchParams { pub sigma: f32, pub weight: f64, pub min_cost: f64, pub max_extra: f64, pub lr0: f32, pub lr_decay: f32, pub lr_floor: f32 }`，`DEFAULT` 见下。
2. `composition.rs`：加 `touches: Vec<Option<TouchSample>>`（与 `shifted` 同步插入/删除/清空）；`pub fn push_touch(&mut self, c: char, touch: Option<TouchSample>)`，`push` 改为调它传 `None`；`pub fn touches(&self) -> &[Option<TouchSample>]`。
3. `engine/composing.rs`：`pub fn push_with_touch(&mut self, c: char, touch: Option<TouchSample>)`，`push(c)` 等价于 `push_with_touch(c, None)`；`Engine` 加字段 `touch_model: TouchModel`、`key_geometry: KeyGeometry`；`engine/setup.rs` 加 `set_key_geometry(&[(char,f32,f32)])`、`set_touch_params(TouchParams)`（后者 `forget_span_cache()` + 清纠错缓存，同 `set_typo_costs`）。
4. `engine/query/lookup.rs::expand_positions`：每个 `pattern` 先算它在作用域里的起始字符下标（新私有函数 `pattern_offsets(scope: &str, patterns) -> Vec<usize>`，跳过分隔符 `'`；**要先确认 patterns 是否就是作用域的连续切片**），对 `TypoKind::Substitute` 变体找出不同的那一位 `j`，取 `touches[offset + j]`，代价改为
   `self.typo_costs.typo_cost(kind, accepted) + touch_model.substitute_adjustment(geometry, typed_j, intended_j, touch)`。
   **注意 `SpanCache` 的键现在要能区分触点**：触点变了缓存要作废——最简单是 `push_with_touch` 带触点时照常（新键本来就会改 span），但退格后同一拼音再敲、触点不同，`SpanCache::key(span)` 相同会命中旧值。改法：把每位置的代价并进 key（`SpanCache::key` 已按 span 文本算，扩成含 alternatives 的代价位），或在 `push_with_touch` 里 `forget_span_cache()`（简单，代价是每键丢缓存，需测延迟）。
5. 学习：`engine/commit/mod.rs` 在 `record_typo` 循环旁，`!self.private` 时对 `consumed` 范围的每个字母位置 `p`（有触点的）求「本意字母」：该音节在 `accepted_typos` 里是 Substitute（同长一位不同）时取 intended 的那一位，否则取 typed 字母；该音节是 Transpose/Extra/Missing 时**整音节跳过**（对不齐）。然后 `touch_model.learn(intended, rel)`，其中 `rel = (touch.dx, touch.dy) − geometry.offset(typed, intended)`（换算成相对本意键中心）。
6. 落盘：`qingjian-learning` 加 `touch_store.rs`（或在 `FrequencyLearner` 旁新类型 `TouchStore`）读写 `user-touch.tsv`：`字母\tmean_x\tmean_y\tvar_x\tvar_y\tcount`，26 行以内，`dirty` 时随学习数据一起存；「清除学习数据」一并删。Core 的 `TouchModel` 只管数值，文件 IO 留在 learning crate（与 `user-typos.tsv` 的分工一致）。
7. 桥：`qingjian-cloud-bridge/src/lib.rs` 加 `qj_push_touch(session, c: u32, dx: f32, dy: f32)`（NaN 表示无触点），`session/mod.rs` 加 `push_touch`；`qj_set_key_geometry(session, *const f32 /*26×2*/)`。
8. iOS 壳：
   - `KeyTouchView`：`onRelease` 改成 `(Int, Bool, CGPoint?)`，传 `origins[id]`（**按下点**；键在按下时就定了，抬起点会被拖动带偏）换算到键区坐标。
   - `KeyboardLayout.slots` 旁算 `pitchX = cx(w) − cx(q)`、`pitchY = cy(a) − cy(q)`（与 RefIME 同定义），布局变化时调一次 `engine.setKeyGeometry`（26 个中心 / 键距）。
   - `KeyboardModel.typeLetter(_:touch:)`：`dx = (p.x − slot.cell.midX)/pitchX`，`dy = (p.y − slot.cell.midY)/pitchY`，调 `engine.push(letter, touch:)`。Shift 大写、符号、⌫ 不传。
   - 用户文档：`docs/user/` 对应 iOS 键盘页加一句「键盘会按你的手势习惯微调纠错，数据只存本机」（用户能感知的行为改了，同提交改）。

代价公式（`substitute_adjustment`）：

```
ℓ_k(t)   = −‖t − (G(typed→k) + m_k)‖² / (2σ²)        // t 为相对 typed 中心的触点；G 为几何偏移；m_k 为 k 的个人均值
LLR(t)   = ℓ_typed(t) − ℓ_intended(t)                 // = −ln P(intended|t) + ln P(typed|t)，softmax 归一项抵消
LLR₀     = LLR(m_typed)                                // 触点恰在用户惯常落点时的值
adj      = weight · (LLR(t) − LLR₀)
cost     = clamp(substitute_base + adj, min_cost, substitute_base + max_extra)
```

即任务原文「邻键代价 = −log P(邻键|触点)」相对 typed 键的版本；触点在惯常落点时 `adj = 0`，代价与现在**逐位相同**（回放零变化可验证），越偏向邻键越便宜。

常数初值（青简自己重扫）：`sigma = 0.5`（RefIME 个人 std 初值，键距单位；RefIME 打分 σ 来自数据文件未知）、`weight = 1.0`、`min_cost = 1.0`、`max_extra = 2.0`、`lr0 = 0.01`、`lr_decay = 0`、`lr_floor = 0.01`（= RefIME 无参数时的 0.01），alpha 夹在 `[1e-4, 0.1]`（RefIME）。

### 4. 复刻步骤

1. `touch/` 模块四个类型 + 单测（纯数值，不碰引擎）。
2. `Composition` 加 `touches`，`push_touch` / 退格 / 清空 / 光标插入同步；单测与 `shifted` 同样的边界。
3. `Engine::push_with_touch`、`set_key_geometry`、`set_touch_params`；`expand_positions` 接代价调整；处理 `SpanCache`。
4. 上屏学习（私密跳过）+ `user-touch.tsv` 读写。
5. `apps/cli`：`--tune` 加 `touch-sigma` / `touch-weight` / `touch-min` / `touch-max`（`tuning.rs::KEYS` 与 `apply`）；回放日志没有触点，冻结日志上应**零变化**（这本身是第一道验收）。
6. 桥 + iOS 壳；真机 200 字开关对比（`docs/plan/model-eval.md` 写法：首选命中率、退格次数）。
7. `docs/notes/crate-notes.md` 记 touch 模块要点与常数；`docs/user/` 改页。

单测清单（输入 → 期望）：

- `KeyStats::update`：初值 0，`(0.3,−0.1)` 喂 1 次、alpha 0.01 → mean = (0.003, −0.001)，count 1。
- alpha 夹取：传 0.5 → 按 0.1 更新；传 0 → 按 1e-4。
- 收敛：对 `g` 喂 600 次 `(0.3,−0.1) + N(0, 0.05²)`（固定种子）→ `|mean − (0.3,−0.1)| < 0.02`（(1−0.01)^600 ≈ 0.0024 残差）。
- 几何：`ios_qwerty().offset('f','g') == (1.0, 0.0)`；`offset('q','a') == (0.5, 1.0)`；`offset('a','z') == (1.0, 1.0)`。
- 无触点：`expand_positions` 的代价与改前一致（快照对比 `substitute = 5.0`）。
- 触点在惯常落点（m_f）：`substitute_adjustment('f','g', m_f) == 0`。
- 触点在 f/g 交界 `dx = +0.5`：`adjustment('f','g') < 0`，`adjustment('f','d') > 0`。
- 夹取：`dx = +0.95`（几乎在 g 中心）→ `cost == min_cost`；`dx = −0.9` → `cost == base + max_extra`。
- 学习对齐：输入 `fe` 触点 `[(0.45,0),(0,0)]`，选了 `ge`（Substitute f→g）→ 学到 `g` 的样本 `(0.45−1.0, 0) = (−0.55, 0)`、`e` 的 `(0,0)`；`f` 不变。
- 换位/多键/少键的音节不学：`shou` 选 `shuo` → 该音节 4 个位置都不更新。
- 私密：`set_private(true)` 后上屏 → 模型与 `dirty` 都不变。
- TSV 往返：`from_tsv(to_tsv(m)) == m`；坏行计入跳过数。

### 5. 未知

- `EmitInputEvent.query_str` 的来源（原始按键串还是纠正后拼写）与 `learnable@+0x74` 的置位条件：没找到构造点（订正：D3 取证确认 `EmitInputEvent+0x74` 不是 learnable，是内嵌 `KeyBoardLayout` 的 valid 字段，即「这次上屏带有效键盘布局」；见 [d3-learning-gate.md](d3-learning-gate.md)。）。下一步：在 `searchers_v2` / `session` 里找写 `+0x74` 的 `strb`，或从 `lsignal::signal<void(EmitInputEvent const&)>` 的 emit 调用点反查（`CloudReportEmitInput::Impl::OnEmitInput` 也订阅同一事件，可对照其 `BuildItemFromEmitInput` 的字段名）。
- `DynamicCorrectionScoreParams` 的 6 个值、`PersonalDictParamsProto` 的 3 个学习率值：在随包数据文件里，本机无该文件。
- `layout.flags & 7` 的含义。
- 动态热区（`KeyboardHotArea`）的预测与下发机制未读。

---

## B3 beam 宽调度

### 1. RefIME 怎么做的

#### 1.1 调度器是 `std::function<size_t(size_t)>`，构造时预算一张表

所有带表的 functor（Cosine / Exp / WarmUp*）在构造函数里把 `width(i)` 预算成 `vector<size_t>`，**最多 100 项，遇到第一个 ≤ min 的值（压成 min 放进去）就停**；`operator()(i)` = `i < len ? table[i] : table.back()`（空表返回 0）[确：Cosine op @0x100a59280、Exp op @0x100a5adf8、WarmUpCosine op @0x100a59b3c、WarmUpNegExp op @0x100a5a3d8，四个函数体相同]。线性 / 非线性两种不建表、现算。所有构造都断言 `min ≤ init && min > 0`（串 `init_beam_size >= min_beam_size && min_beam_size > 0`，实现是无符号 `min−1 < init`）[确]。

以下 `floor` = C 的 `fcvtzu`（截断取整）；`init`/`min`/`peak`/`W`/`K` 为 size_t，`rate`/`period` 为 float。

| functor（AB 类型码） | 构造 / 公式 | 地址 |
|---|---|---|
| 常量（`0,N`；capnp kind 0） | `width(i) = N` | lambda @0x100a5a4cc（`ldr w0,[x0,#8]`） |
| `DecayBeamSizeSchedulerFunctor`（`1`，线性；capnp kind 2） | `width(i) = (rate·i + min < init) ? init − floor(rate·i) : min`（float 运算） | ctor 0x100a5a534，op 0x100a5a6c8 |
| `ExpDecayBeamSizeSchedulerFunctor`（`2`；capnp kind 1） | `w₀ = init；w_i = max(min, floor(w_{i−1} · expf(−rate·i)))` —— **累乘**，实际 ≈ `init·exp(−rate·i(i+1)/2)` | ctor 0x100a5a75c（`fneg s8`、`ucvtf s9,x21` 用上一项）|
| `NonlinearDecayBeamSizeSchedulerFunctor`（`3`；capnp kind 3） | `if (rate·i + min ≥ init) return min; d = (i < K) ? floor(rate·(2K − i)) : floor(rate·i); return (i<K && min + d > init) ? min : init − d` —— 帐篷形：i<K 时随 i 变宽，i=K 最宽，之后线性变窄 | ctor 0x100a5ae8c，op 0x100a5b024 |
| `CosineDecayBeamSizeSchedulerFunctor`（`4`） | `w_i = max(min, floor(init · (0.5·cos(π/period · i) + 0.5)))`（double） | ctor 0x100a58a90（π=0x400921fb54442d18，`_cos`） |
| `WarmUpCosineDecayBeamSizeSchedulerFunctor`（`5`），ctor `(init, min, peak, W, period)` | `W = max(W,1)`；`i ≤ W：v = init + i·(peak − init)/W`（整数除）；`i > W：v = floor(peak·(0.5·cos(π/period·(i−W)) + 0.5))`；`w_i = max(min, v)` | ctor 0x100a59314 |
| `WarmUpNegativeExpDecayBeamSizeSchedulerFunctor`（`6`），ctor `(init, min, peak, W, rate)` | `i ≤ W` 同上线性升；`i > W：v = floor(peak · (2 − expf(rate·(i−W))))`；`w_i = max(min, v)` | ctor 0x100a59bd0（`fmov s10,#2.0`，`_expf`） |

边角：带表的几种在 `i=0` 就 ≤ min 时表只有 `[min]`（WarmUp 在 `init == min` 时就是这样，等于常量 min）[确：循环条件]。

#### 1.2 工厂与默认值：`BeamSizeSchedulerFactory`

- `Create(HyperParams const&, EnvContext*)` @0x100a578a8，在 `SentenceSearcherV3` 构造里调（@0x100a57114）[确]：
  1. 先读 AB 参数 `mmkb_cli_engine_beam_size_scheduler`（串 @0x102cd22bd），逗号分隔：
     - `"0,N"` → 常量 N；
     - `"<类型 1..6>,init,min,rate[,…]"`：`min = max(min, 2)`、`init = max(init, min)`；类型 1/2/4 要 4 段，3 要 5 段（第 5 段 K），5/6 要 6 段（`init,min,peak,W,period|rate`，此时第 3/4/5 段分别是 peak/W/period）[确：跳表 @0x102e8d4c5 = `00 5b 35 4e 14 68`]。
  2. 否则读 `HyperParams::BeamSizeSchedulerParams()`（capnp，union u16 判别）：0 = 常量（u32，0 则抛错）、1 = Exp(u32 init, u32 min, f32 rate)、2 = 线性 Decay(同)、3 = Nonlinear(+u32 K) [确：跳表 @0x102e8d4cb + 各分支调用的 ctor]。
  3. **线性分支里，若 `EnvContext+0x278 == 2`（低性能态），`init >>= 1`**（`lsr w1, w21, w27`）[确 @0x100a57fc0..0x100a581ac]。
- `Create(string_view name)` @0x100a58600 —— 硬编码的命名预设，**这就是 RefIME 写死的一组参考值** [确：立即数与常量 @0x102e8d440 = (20,2)(50,2)]：

  | 名字 | 调度 |
  |---|---|
  | `exponential_decay` | Exp(init 20, min 2, rate 0.2) → 表 `[20, 16, 10, 5, 2]` |
  | `non_linear_decay` | Nonlinear(init 20, min 2, rate 0.2, K 6) |
  | `decay` | 线性(init 50, min 2, rate 0.5) |
  | `const` / 其它 | 常量 100 |

#### 1.3 在 `SentenceSearcherV3` 里哪一步调

```c
// SentenceSearcherV3::FeedBeam(int pos, gen, layout, coords, ...) @0x100a37688
if (beams[pos] 已非空) return;                    // this+0x98：vector<Beam>（32 字节/个），按 pos 缓存
size_t B = scheduler(pos - this->start /*+0xb0*/);  // this+0x78 处的 std::function，@0x100a377a4
GenerateBeamsV2(pos, B, gen, layout, coords, ...);  // @0x100a3782c
```

- `pos` 来自 `Searcher::FeedBeam(unsigned pos, …)` @0x100a2fca0，由 `searchAndGetResultImpl` 逐位置调 [确]；`pos` 是输入串位置（音节图节点 = 字符下标）[推]；`start@+0xb0` 是本次搜索的起点 [推：已上屏/固定前缀的长度]。**所以 i 是「相对起点的输入字符位置」，不是音节数。**

#### 1.4 各比例怎么乘：子配额，不是乘总宽

`GenerateBeamsV2(int pos, int B, …)` @0x100a02b60 [确 @0x100a03fb0..0x100a040c8]：

```c
float r_jp = this->presort.jianpin_ratio /*+0x1d0*/;   if (r_jp < 1e-4) r_jp = 0.1;
float r_uw = BeamSizeUserWordsRatioParams(is_t9).f[0];  if (r_uw < 1e-4) r_uw = 0.06;   // 0x3d75c28f
float r_en = BeamSizeEnRatioParams(is_t9).f[0];         if (r_en < 1e-4) r_en = 0.1;
int jp_quota = max(1, (int)(r_jp * B));
int uw_quota = min(max(1, (int)(r_uw * B)), 10);
int en_quota = max(1, (int)(r_en * B));
```

- 候选音节先被 `SplitSyllablesToChunks` 分成四块（简拼 / 用户词 / 英文 / 其余），每块一个 `PathV2AndSyllableCombineHelper`（best-first 归并），**简拼块最多取 jp_quota 条、用户词块最多 uw_quota、英文块最多 en_quota（英文块最多扫 2·en_quota 条去找带 +0x1f0 标志的那条替换最后一条）、主块填到 B** [确：循环上界 w26/w27/w23，`lsl w21, w23, #1`；主块上界推]。即「比例 = 在 beam 里给这类路径保留的名额」，保证简拼、用户词、英文路径不被主路径挤光。
- `r_jp` 来自 `BeamSearchPreSortParamReader`（this+0x1c8）@0x100d81fe4：AB 参数 `mmkb_cli_engine_beam_search_pre_sort_param`（`"<模式>,a,b,ratio;…"`，模式 `1`=T9、`0`=全键盘），否则 `BeamSearchPreOrderParams(is_t9)` 的两个 u32 + `BeamSizeJianPinRatioParams(is_t9)` 的 f32 [确]。
- 最小宽度：调度器的 `min`（AB 路径强制 ≥ 2）；各子配额 ≥ 1。

#### 1.5 FrontierShrink / LeavesShrink：每条音节边的词典游标上限

- `graphs_v3::GetCursorLimit(bool leaves, HyperParams const&, EnvContext const&)` @0x100d090ac [确]：

  ```c
  if (leaves) {               // LeavesShrinkParams：{u32 full, u32 t9}
      lim = is_t9 ? p.u32[1] : p.u32[0];  if (!lim) lim = 100;
  } else if (env->perf_state /*+0x278*/ >= 1) {
      lim = is_t9 ? 30 : 50;               // 降级态写死
  } else {                    // FrontierShrinkParams：{u32 full, ?, u32 t9}
      lim = is_t9 ? p.u32[2] : p.u32[0];  if (!lim) lim = is_t9 ? 30 : 100;
  }
  ```
- `graphs_v2::SyllableEdge::ShrinkFrontiers` @0x1009a4234 / `ShrinkLeaves` @0x1009a35f0：把这条音节边上的 trie 搜索节点（frontier = 还在往下走的前缀游标；leaf = 已成词的结果）包成 `SyllableShrinker::SortItem`，`SyllableShrinker::Shrink` @0x1009a89e8 排序截断到上限，同时按 `MinSyllableEdgeKeepParams` 保底 [确：调用链；排序键未读]。即 beam 之外的第二道剪枝：**限的是每段拼音进词图的候选数**，对应青简的 `SPAN_POOL` / `ABBREVIATED_SPAN_CANDIDATES`。

#### 1.6 低性能态 `EnvContext+0x278`

`EnvContext::IsLowPerformance` @0x100655630：比较 +0x378/+0x380/+0x388 三个 double 与阈值，**带滞回**（已在低性能态 `==2` 时用高阈值退出）：35/25、100/75、200/150（态 2 / 其它）；+0x390 < 0 也算低 [确：常数；推：这三个是逐键耗时的分位数，单位 ms]。低性能态下：线性调度 init 减半、Frontier 上限降到 50（T9 30）。

### 2. 证据表

| 结论 | 置信度 | 地址 / 字符串 |
|---|---|---|
| 调度器类型是 `function<size_t(size_t)>` | 确 | `__func<…, unsigned long (unsigned long)>` 符号 |
| 带表 functor 预算 ≤100 项、遇 min 截止、越界取末项 | 确 | 0x100a58c10 `cmp x23,#0x62`；op 0x100a59280 |
| 六种公式（见表） | 确 | 各 ctor/op 地址 |
| Exp 是累乘 | 确 | 0x100a5a7c4..0x100a5a7e0（x21 被回写） |
| AB 参数名与格式、min≥2 | 确 | `mmkb_cli_engine_beam_size_scheduler`；0x100a57a70..0x100a57a88 |
| 命名预设 20/2/0.2、50/2/0.5、K 6、常量 100 | 确 | 0x100a58600；常量 0x102e8d440 |
| 低性能态线性 init 减半 | 确 | 0x100a581ac `lsr w1, w21, w27` |
| 按 `pos − start` 调用 | 确 | 0x100a37778..0x100a377a4 |
| pos 是字符位置 | 推 | Searcher::FeedBeam(unsigned) 逐位置调用 |
| 简拼/用户词/英文是子配额，缺省 0.1/0.06(≤10)/0.1 | 确 | 0x100a03fb0..0x100a040c8；0x100a043f0.. |
| Frontier/Leaves 上限与降级值 30/50/100 | 确 | 0x100d090ac |
| IsLowPerformance 阈值与滞回 | 确（常数）/推（语义） | 0x100655630 |

### 3. 青简落点

现状（`crates/qingjian-core/src/sentence/`）：

- **固定 beam**：`sentence/mod.rs:86 pub const BEAM_WIDTH: usize = 8;`，唯一用处 `viterbi/mod.rs:447 fn prune` 的 `nodes.truncate(BEAM_WIDTH)`。
- 剪枝点：`convert_paths` 里 `for start in 0..n { prune(&mut nodes[start]); … }` 与末尾 `prune(&mut nodes[n])`。**位置是音节下标**（`positions` 每格一个音节），与 RefIME 的字符下标不同：斜率要按音节重扫。
- 其它宽度常数：`SPAN_CANDIDATES` 6、`SPAN_POOL` 16、`ABBREVIATED_SPAN_CANDIDATES` 20（对应 RefIME 的 Frontier/Leaves 上限，不是 beam）；`engine/mod.rs:425 RESCORE_PATHS = 6`。
- 调参入口：`apps/cli/src/tuning.rs`（`KEYS` + `apply`，经 `Engine::set_interpolation` / `set_typo_costs`）；耗时：`apps/cli/src/latency.rs::Latencies`（现在只出 p50/p99）。

改法：

1. 新文件 `crates/qingjian-core/src/sentence/beam_schedule.rs`：

   ```rust
   //! 整句束宽随位置变化：width(i) = max(min, initial − floor(slope·i))，有简拼的输入再乘一个倍率。
   #[derive(Debug, Clone, Copy, PartialEq)]
   pub struct BeamSchedule {
       /// 第 0 个音节位置的束宽。
       pub initial: usize,
       /// 束宽下限（≥ 1）。
       pub min: usize,
       /// 每往后一个音节少几条（可为小数，按 floor 取整，同 RefIME 线性 functor）。
       pub slope: f64,
       /// 输入里有简拼位置时束宽乘的倍率（RefIME 是子配额，见 docs；青简先做倍率）。
       pub abbreviated_ratio: f64,
   }
   impl BeamSchedule {
       /// 现行为：恒 8。
       pub const FIXED: Self = Self { initial: BEAM_WIDTH, min: BEAM_WIDTH, slope: 0.0, abbreviated_ratio: 1.0 };
       pub fn width(&self, position: usize, abbreviated: bool) -> usize {
           let linear = if self.slope * position as f64 + self.min as f64 >= self.initial as f64 {
               self.min
           } else {
               self.initial - (self.slope * position as f64) as usize
           };
           let scaled = if abbreviated { (linear as f64 * self.abbreviated_ratio).round() as usize } else { linear };
           scaled.max(self.min).max(1)
       }
   }
   ```
   `sentence/mod.rs` 加 `mod beam_schedule; pub use beam_schedule::BeamSchedule;`，`BEAM_WIDTH` 保留作 `FIXED` 的值。
2. `viterbi/mod.rs`：`convert` / `convert_whole` / `convert_paths` 加参数 `beam: BeamSchedule`（`convert_paths` 已 `#[allow(clippy::too_many_arguments)]`）；`prune(nodes, width)` 带宽度；`abbreviated_input = positions.iter().any(|p| p.first().is_none_or(|t| !t.complete))` 算一次；
   - 循环里 `prune(&mut nodes[start], beam.width(start, abbreviated_input))`；
   - 末尾 `prune(&mut nodes[n], beam.width(n, abbreviated_input).max(k))` —— **末位置必须 ≥ k**，否则 `RESCORE_PATHS`(6) 条重打分路径凑不齐，神经重排会悄悄变差。
3. `Engine` 加字段 `beam_schedule: BeamSchedule`（缺省 `FIXED`），`engine/setup.rs` 加 `set_beam_schedule` / `beam_schedule()`；`engine/query/converting.rs::paths` 把它传给 `convert_paths`。`SpanCache` 不依赖束宽，不用作废。
4. `apps/cli/src/tuning.rs`：`KEYS` 加 `"beam"`（initial）、`"beam-min"`、`"beam-slope"`、`"beam-abbr"`，`apply` 里组一个 `BeamSchedule` 调 `engine.set_beam_schedule`（`beam`/`beam-min` 解析后 `as usize`，负数报 `TuneError::Value`）。
5. `apps/cli/src/latency.rs`：`summary` 改成 `p50 / p95 / p99 / max`（加 `quantile(0.95)` 与 `max()`）；回放报告再按「音节数 ≥ 6 的长句」单出一行耗时（`replay/report.rs` 的 `sync` 旁加 `sync_long: Latencies`），因为验收看的是长句 p95。
6. 性能档位（任务原文后面的 iOS 档位任务）：壳在低档时 `set_beam_schedule(BeamSchedule { initial: initial / 2, .. })`，对应 RefIME 低性能态线性 init 减半；同理可在低档把 `ABBREVIATED_SPAN_CANDIDATES` 这类每格上限调低（RefIME Frontier 降到 50/30）——这一条要另开常数入口，本任务不做。

常数初值（全部要在冻结日志上重扫）：`FIXED` 不变；扫描网格建议 `beam ∈ {8, 10, 12, 16}` × `beam-min ∈ {4, 6, 8}` × `beam-slope ∈ {0, 0.5, 1, 2}`，`beam-abbr ∈ {1, 1.5, 2}`。RefIME 参考：线性 `decay` 预设 init 50 / min 2 / 每字符 0.5（单位是字符位置，且它的 beam 还要再分子配额，不能直接搬）。

### 4. 复刻步骤

1. `BeamSchedule` + 单测。
2. `viterbi` 接参数；`Engine` 字段 / setter；`converting.rs` 传参。`cargo test -p qingjian-core` 全过（`FIXED` 下零变化）。
3. CLI `--tune beam…`；`Latencies` 加 p95/max 与长句分行。
4. 冻结日志回放：先跑缺省，确认报告与改前逐行相同；再扫网格（四路并行，同 constant-sweep.md 的做法）。
5. 验收：精度（各来源首选命中数）不低于缺省（±0）且长句 p95 下降才合；结果追加到 `docs/notes/constant-sweep.md`，`docs/notes/crate-notes.md:37` 那段更新束宽说明。

单测清单：

- `FIXED.width(i, x) == 8` 对 i ∈ 0..20、x ∈ {true,false}。
- `{initial 12, min 4, slope 1, ratio 1}`：width(0)=12、width(3)=9、width(8)=4、width(20)=4。
- 小数斜率：`slope 0.5`：width(1)=12（floor 0.5=0）、width(2)=11、width(3)=11。
- 边界同 RefIME：`slope·i + min ≥ initial` 时取 min：`{12,4,1}` 的 width(8)：1·8+4=12 ≥ 12 → 4。
- 倍率：`{8,4,0,ratio 1.5}`：width(0,true)=12、width(0,false)=8；`ratio 0.25` 时不低于 min。
- `convert_paths` 末位置：`k = 6`、调度 `min 2` 时 `nodes[n]` 仍留 ≥ 6 条（构造一个末位置有 ≥ 6 条路径的词图，断言返回 6 条）。
- viterbi 回归：`BeamSchedule::FIXED` 下 `viterbi/tests.rs` 现有用例全过（不改断言）。
- `tuning::apply(&["beam=12,beam-slope=1"])` 后 `engine.beam_schedule()` 对应；`beam=-1` 报 `TuneError::Value`；未知名报 `Unknown`。
- `Latencies`：`[1..=100] ms` 的 p95 = 95 ms、max = 100 ms。

### 5. 未知

- capnp 超参里 `BeamSizeSchedulerParams` 的实际取值（数据文件）——即 RefIME 线上到底用哪种调度、什么常数，不知道；只知道硬编码预设与 AB 通道。
- `SentenceSearcherV3+0xb0`（start）的精确语义：未追写入点；下一步看 `SentenceSearcherV3::ResetBeam` @0x100a08260 / `CandidateSelected` @0x100a0831c 里对 `+0xb0` 的 `str`。
- 主块的上界是否就是 B（推测）、英文块 `+0x1f0` 标志的含义：下一步读 0x100a047c8 之后的主块循环与 `PathV2` +0x1f0 的写入点。
- `SyllableShrinker::SortItem` 的排序键（词频？路径分？）：`SortItem::SortItem` @0x1009e2f1c 只看到一个 float，未读来源。
- `IsLowPerformance` 三个 double 的来源与单位（推测是逐键耗时分位数 ms）：下一步找写 `EnvContext+0x378` 的函数。
