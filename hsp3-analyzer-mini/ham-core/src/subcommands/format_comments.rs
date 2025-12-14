use crate::{
    token::{tokenize, TokenKind},
    utils::rc_str::RcStr,
};

pub fn format_comments(text: &str) -> String {
    let text_len = text.len();
    let text = RcStr::from(text);
    let tokens = tokenize(1, text.clone());

    let mut output = String::with_capacity(text_len);
    for token in tokens {
        match token.kind {
            TokenKind::Comment if token.text.starts_with("//") => {
                assert!(!token.text.contains("\n"), "コメントは改行を含まないはず");
                let slash = token.text.chars().take_while(|&c| c == '/').count();
                let space = token.text[slash..]
                    .chars()
                    .take_while(|&c| c == ' ')
                    .count();
                let tab = token.text[slash..]
                    .chars()
                    .take_while(|&c| c == '\t')
                    .count();
                let rest = &token.text[slash + space.max(tab)..];

                // "// ----..." みたいなやつ(境界線)
                if slash == 2 && space == 1 && rest.len() >= 10 && rest.chars().all(|c| c == '-') {
                    output += "; -";
                    output += rest;
                    continue;
                }
                if slash == 2 && space == 1 && rest.len() >= 10 && rest.chars().all(|c| c == '=') {
                    output += "; =";
                    output += rest;
                    continue;
                }

                let mut n = slash + space;

                if slash == 3 {
                    // スラッシュ3つはドキュメンテーションコメントとみなす。
                    output += ";;";
                    n -= 2;
                } else {
                    output += ";";
                    n -= 1;
                };

                if tab >= 1 {
                    // タブによるスペースは調整しない。
                    for _ in 0..tab {
                        output += "\t";
                    }
                } else if space == 1 {
                    // もともとスペースが1個ならスペースによる桁合わせは行われていないとみなして、1つだけスペースを入れる。
                    output += " ";
                } else if space >= 2 {
                    // 桁を合わせる。
                    for _ in 0..n {
                        output += " ";
                    }
                }

                output += rest;
                continue;
            }
            _ => {}
        }

        output += &token.text;
    }

    output
}

#[cfg(test)]
mod inline_tests {
    use super::*;
    use expect_test::expect;

    const INPUT: &'static str = include_str!("../../../tests/format-comments/slash.hsp");

    #[test]
    fn format_test() {
        let output = format_comments(INPUT);

        expect![[r##"
; 普通のコメント
;       桁そろえされたコメント
;

mes "命令" ; 命令の後のコメント

;; ドキュメントコメント
;;
;;      桁揃えされたドキュメンテーションコメント
#deffunc doc

;	タブのあるコメント
;;	タブのあるドキュメンテーションコメント

/* v                                             v */
; ------------------------------------------------
; セクション見出し
; ------------------------------------------------

/* v                                             v */
; ================================================
"##]]
        .assert_eq(&output);
    }
}
