use crate::analysis::ALoc;
use crate::token::{TokenData, TokenKind};
use std::fmt::Debug;

/// トリビアでないトークンに、前後のトリビアをくっつけたもの。
#[derive(Clone)]
#[must_use]
pub(crate) struct PToken {
    pub(crate) leading: Vec<TokenData>,
    pub(crate) body: TokenData,
    pub(crate) trailing: Vec<TokenData>,
}

impl PToken {
    pub(crate) fn kind(&self) -> TokenKind {
        self.body.kind
    }

    pub(crate) fn body_text(&self) -> &str {
        self.body.text.as_str()
    }

    /// このトークンの末尾の位置 (後続トリビアを含む)
    pub(crate) fn behind(&self) -> ALoc {
        match self.trailing.last() {
            Some(last) => last.loc.behind(),
            None => self.body.loc.behind(),
        }
    }

    pub(crate) fn is_jump_modifier(&self) -> bool {
        self.kind() == TokenKind::Ident
            && match self.body.text.as_str() {
                "goto" | "gosub" => true,
                _ => false,
            }
    }

    pub(crate) fn from_tokens(tokens: Vec<TokenData>) -> Vec<PToken> {
        let empty_text = {
            let eof = tokens.last().unwrap();
            eof.text.slice(0, 0)
        };

        let mut tokens = tokens.into_iter().peekable();
        let mut p_tokens = vec![];
        let mut leading = vec![];
        let mut trailing = vec![];

        loop {
            // トークンの前にあるトリビアは先行トリビアとする。
            while tokens.peek().map_or(false, |t| t.kind.is_leading_trivia()) {
                leading.push(tokens.next().unwrap());
            }

            let body = match tokens.next() {
                Some(body) => {
                    assert!(!body.kind.is_leading_trivia());
                    body
                }
                None => break,
            };

            while tokens.peek().map_or(false, |t| t.kind.is_trailing_trivia()) {
                trailing.push(tokens.next().unwrap());
            }

            p_tokens.push(PToken {
                leading: leading.split_off(0),
                body,
                trailing: trailing.split_off(0),
            });

            // 改行の前に文の終わりを挿入する。
            if tokens.peek().map_or(false, |t| t.kind == TokenKind::Eol) {
                let loc = p_tokens.last().map(|t| t.behind()).unwrap_or_default();

                p_tokens.push(PToken {
                    leading: vec![],
                    body: TokenData {
                        kind: TokenKind::Eos,
                        text: empty_text.clone(),
                        loc,
                    },
                    trailing: vec![],
                });
            }
        }

        assert!(leading.is_empty());
        assert!(trailing.is_empty());

        p_tokens
    }
}

impl Debug for PToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self
            .leading
            .iter()
            .any(|token| token.kind != TokenKind::Eol && token.kind != TokenKind::Space)
        {
            let mut list = f.debug_list();
            list.entries(
                self.leading
                    .iter()
                    .filter(|token| token.kind != TokenKind::Eol && token.kind != TokenKind::Space),
            );
            list.finish()?;
            write!(f, "> ")?;
        }

        Debug::fmt(&self.body, f)?;

        if self
            .trailing
            .iter()
            .any(|token| token.kind != TokenKind::Eol && token.kind != TokenKind::Space)
        {
            write!(f, " <")?;
            let mut list = f.debug_list();
            list.entries(
                self.trailing
                    .iter()
                    .filter(|token| token.kind != TokenKind::Eol && token.kind != TokenKind::Space),
            );
            list.finish()?;
        }

        Ok(())
    }
}
