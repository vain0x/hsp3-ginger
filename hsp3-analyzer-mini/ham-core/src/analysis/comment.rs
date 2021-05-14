use super::*;

fn char_is_ornament_comment(c: char) -> bool {
    c.is_control() || c.is_whitespace() || c.is_ascii_punctuation()
}

/// 装飾コメント (// ---- とか) や空行など
pub(crate) fn str_is_ornament_comment(s: &str) -> bool {
    s.chars().all(char_is_ornament_comment)
}

fn trim_comment_leader(s: RcStr) -> RcStr {
    for prefix in &["/// ", "///", "// ", "//", "; ", ";"] {
        if s.starts_with(prefix) {
            return s.slice(prefix.len(), s.len());
        }
    }
    s
}

pub(crate) fn calculate_details(comments: &[RcStr]) -> ASymbolDetails {
    let mut description = None;
    let mut documentation = vec![];

    let mut y = 0;

    for comment in comments {
        y += 1;

        // 装飾コメントや空行を無視
        let t = comment.as_str().trim();
        if str_is_ornament_comment(t) {
            continue;
        }

        // 最初の行は概要
        description = Some(trim_comment_leader(comment.clone()));
        break;
    }

    for line in &comments[y..] {
        // 装飾コメントや空行を無視
        let t = line.as_str().trim();
        if str_is_ornament_comment(t) {
            y += 1;
            continue;
        }
        break;
    }

    // 残りの行はドキュメンテーション
    if y < comments.len() {
        for comment in &comments[y..] {
            let comment = trim_comment_leader(comment.clone());
            documentation.push(comment.as_str().trim_end().to_string());
        }
    }

    ASymbolDetails {
        desc: description,
        docs: documentation,
    }
}

pub(crate) fn collect_comments(leader: &PToken) -> Vec<RcStr> {
    let leading = &leader.leading;

    // leadingの末尾にあるトークンのうち、定義位置との間に空行を挟まないものの個数。
    let n = leading
        .iter()
        .rev()
        .take_while(|t| {
            t.kind != TokenKind::Newlines || t.text.chars().filter(|&c| c == '\n').count() <= 1
        })
        .count();

    leading[leading.len() - n..]
        .iter()
        .filter_map(|t| {
            if t.kind == TokenKind::Comment && !str_is_ornament_comment(&t.text) {
                Some(t.text.clone())
            } else {
                None
            }
        })
        .collect()
}
