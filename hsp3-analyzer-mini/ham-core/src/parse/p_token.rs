use super::*;

/// トリビアでないトークンに、前後のトリビアをくっつけたもの。
#[derive(Clone)]
#[must_use]
pub(crate) struct PToken {
    pub(crate) leading: RcSlice<TokenData>,
    pub(crate) body: RcItem<TokenData>,
    pub(crate) trailing: RcSlice<TokenData>,
}

impl PToken {
    pub(crate) fn kind(&self) -> TokenKind {
        self.body.kind
    }

    pub(crate) fn body_text(&self) -> &str {
        self.body.text.as_str()
    }

    pub(crate) fn body_pos(&self) -> Pos {
        self.body.loc.start()
    }

    pub(crate) fn body_pos16(&self) -> Pos16 {
        Pos16::from(self.body.loc.start())
    }

    pub(crate) fn ahead(&self) -> Loc {
        match self.leading.first() {
            Some(first) => first.loc.ahead(),
            None => self.body.loc.ahead(),
        }
    }

    /// このトークンの末尾の位置 (後続トリビアを含む)
    pub(crate) fn behind(&self) -> Loc {
        match self.trailing.last() {
            Some(last) => last.loc.behind(),
            None => self.body.loc.behind(),
        }
    }

    pub(crate) fn iter<'a>(&'a self) -> impl Iterator<Item = &'a TokenData> + 'a {
        self.leading
            .iter()
            .chain(iter::once(self.body.as_ref()))
            .chain(self.trailing.iter())
    }

    pub(crate) fn from_tokens(tokens: RcSlice<TokenData>) -> Vec<PToken> {
        assert!(!tokens.is_empty());

        let mut p_tokens = vec![];
        let mut index = 0;

        loop {
            let leading_start = index;

            // トークンの前にあるトリビアは先行トリビアとする。
            while tokens
                .get(index)
                .map_or(false, |t| t.kind.is_leading_trivia())
            {
                index += 1;
            }
            let leading = tokens.slice(leading_start, index);

            let body = match tokens.item(index) {
                Some(body) => {
                    assert!(!body.kind.is_leading_trivia());
                    index += 1;
                    body
                }
                None => {
                    debug_assert!(leading.is_empty());
                    break;
                }
            };

            let trailing_start = index;
            while tokens
                .get(index)
                .map_or(false, |t| t.kind.is_trailing_trivia())
            {
                index += 1;
            }
            let trailing = tokens.slice(trailing_start, index);

            p_tokens.push(PToken {
                leading,
                body,
                trailing,
            });

            // 行末に文の終わりを挿入する。
            if tokens.get(index).map_or(false, |t| {
                t.kind == TokenKind::Newlines || t.kind == TokenKind::Eof
            }) {
                let loc = p_tokens.last().map(|t| t.behind()).unwrap();

                p_tokens.push(PToken {
                    leading: [].into(),
                    body: TokenData {
                        kind: TokenKind::Eos,
                        text: "".into(),
                        loc,
                    }
                    .into(),
                    trailing: [].into(),
                });
            }
        }

        p_tokens
    }
}

impl Debug for PToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.leading.iter().any(|token| !token.kind.is_space()) {
            let mut list = f.debug_list();
            list.entries(self.leading.iter().filter(|token| !token.kind.is_space()));
            list.finish()?;
            write!(f, "> ")?;
        }

        Debug::fmt(&self.body, f)?;

        if self.trailing.iter().any(|token| !token.kind.is_space()) {
            write!(f, " <")?;
            let mut list = f.debug_list();
            list.entries(self.trailing.iter().filter(|token| !token.kind.is_space()));
            list.finish()?;
        }

        Ok(())
    }
}
