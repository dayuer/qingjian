//! 术语表的一行：纯术语，或畅译词库那种「错的片段 → 对的片段」纠错对，可带来源会议与确认状态。

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// 识别错的片段；纯术语为 `None`。
    pub wrong: Option<String>,

    pub right: String,

    /// 学自哪场会议；`None` 为全局。
    pub scope: Option<String>,

    /// 状态列是 `candidate` 时为 `false`，其余（含没写）为 `true`。
    pub confirmed: bool,
}

impl Entry {
    /// 解析术语表：有 Tab 的行是 `错<Tab>对[<Tab>会议[<Tab>状态]]`（错可以空），
    /// 没 Tab 的行按逗号、顿号拆成多条纯术语；`#` 开头的行跳过。
    pub fn parse_all(text: &str) -> Vec<Self> {
        let mut entries = Vec::new();
        for line in text.lines() {
            if line.trim_start().starts_with('#') {
                continue;
            }
            if line.contains('\t') {
                let mut columns = line.split('\t').map(str::trim);
                let wrong = columns.next().unwrap_or_default();
                let right = columns.next().unwrap_or_default();
                if right.is_empty() {
                    continue;
                }
                let scope = columns.next().filter(|s| !s.is_empty());
                let confirmed = columns.next() != Some("candidate");
                entries.push(Self {
                    wrong: (!wrong.is_empty()).then(|| wrong.to_owned()),
                    right: right.to_owned(),
                    scope: scope.map(str::to_owned),
                    confirmed,
                });
                continue;
            }
            for term in line.split([',', '，', '、']) {
                let term = term.trim();
                if !term.is_empty() {
                    entries.push(Self {
                        wrong: None,
                        right: term.to_owned(),
                        scope: None,
                        confirmed: true,
                    });
                }
            }
        }
        entries
    }
}
