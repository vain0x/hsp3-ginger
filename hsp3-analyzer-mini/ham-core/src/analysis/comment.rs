use super::ASymbolDetails;
use crate::utils::rc_str::RcStr;

fn char_is_ornament_comment(c: char) -> bool {
    c.is_control() || c.is_whitespace() || c.is_ascii_punctuation()
}

/// 装飾コメント (// ---- とか) や空行など
fn str_is_ornament_comment(s: &str) -> bool {
    s.chars().all(char_is_ornament_comment)
}

fn calculate_details(comments: &[RcStr]) -> ASymbolDetails {
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
        description = Some(comment.clone());
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
        documentation.push(
            comments[y..]
                .into_iter()
                .map(|s| s.as_str().trim())
                .collect::<Vec<_>>()
                .join("\r\n"),
        );
    }

    ASymbolDetails {
        desc: description,
        docs: documentation,
    }
}
