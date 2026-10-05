//! 文本小工具：汉字判断、字错率用的归一化与编辑距离、识别结果到参考文本的逐字对齐。

pub fn is_han(c: char) -> bool {
    matches!(c, '\u{3400}'..='\u{4DBF}' | '\u{4E00}'..='\u{9FFF}' | '\u{F900}'..='\u{FAFF}' | '\u{20000}'..='\u{2FA1F}')
}

/// 字错率只看汉字与字母数字：识别结果的标点来自标点模型，与参考文本的标点不必对齐。
pub fn normalize(text: &str) -> Vec<char> {
    text.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

pub fn edit_distance(a: &[char], b: &[char]) -> usize {
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    let mut current = vec![0; b.len() + 1];
    for (i, x) in a.iter().enumerate() {
        current[0] = i + 1;
        for (j, y) in b.iter().enumerate() {
            let substitute = previous[j] + usize::from(x != y);
            current[j + 1] = substitute.min(previous[j + 1] + 1).min(current[j] + 1);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[b.len()]
}

/// `a` 每个字在 `b` 里对上的位置（相同或替换），被删掉的字是 `None`。按编辑距离回溯，
/// 精修稿删了语气词、改了标点也能对齐到术语那几个字上。
pub fn align(a: &[char], b: &[char]) -> Vec<Option<usize>> {
    let (n, m) = (a.len(), b.len());
    let mut table = vec![vec![0usize; m + 1]; n + 1];
    for (i, row) in table.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in table[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=n {
        for j in 1..=m {
            let substitute = table[i - 1][j - 1] + usize::from(a[i - 1] != b[j - 1]);
            table[i][j] = substitute.min(table[i - 1][j] + 1).min(table[i][j - 1] + 1);
        }
    }
    let mut mapping = vec![None; n];
    let (mut i, mut j) = (n, m);
    // 同字优先对上；不同字时先试删 / 插，打平才算替换，免得句尾的语气词被对到术语上
    while i > 0 && j > 0 {
        let same = a[i - 1] == b[j - 1];
        if same && table[i][j] == table[i - 1][j - 1] {
            mapping[i - 1] = Some(j - 1);
            i -= 1;
            j -= 1;
        } else if table[i][j] == table[i - 1][j] + 1 {
            i -= 1;
        } else if table[i][j] == table[i][j - 1] + 1 {
            j -= 1;
        } else {
            mapping[i - 1] = Some(j - 1);
            i -= 1;
            j -= 1;
        }
    }
    mapping
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aligns_across_dropped_fillers() {
        let a: Vec<char> = "呃这个基限啊".chars().collect();
        let b: Vec<char> = "这个基线".chars().collect();
        let mapping = align(&a, &b);
        assert_eq!(mapping[0], None);
        assert_eq!(mapping[3], Some(2));
        assert_eq!(mapping[4], Some(3));
    }
}
