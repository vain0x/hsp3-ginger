//! フォーマッティング
//!
//! 字下げや空白を調整する。

use super::*;
use crate::{assists::to_lsp_range, lang_service::docs::Docs, parse::*};
use lsp_types::{TextEdit, Url};

fn index_range(range: Range) -> std::ops::Range<usize> {
    range.start().index as usize..range.end().index as usize
}

/// 命令がどのくらい字下げを変化させるか
///
/// 結果の1つ目は命令の直前での字下げの変化、2つ目は命令の直後での字下げの変化。
fn delta(s: &str) -> Option<(i32, i32)> {
    let it = match s {
        "switch" => (0, 2),
        "swend" => (-2, 0),
        "case" | "default" => (-1, 1),
        "repeat" | "foreach" | "for" | "while" | "do" => (0, 1),
        "loop" | "next" | "wend" | "until" => (-1, 0),
        _ => return None,
    };
    Some(it)
}

fn leading_in_same_line_is_all_blank(token: &PToken) -> bool {
    token
        .leading
        .iter()
        .rev()
        .take_while(|t| t.kind != TokenKind::Newlines)
        .all(|t| t.kind == TokenKind::Blank)
}

fn trailing_is_all_blank(token: &PToken) -> bool {
    token.trailing.iter().all(|t| t.kind == TokenKind::Blank)
}

fn leading_blank_range(token: &PToken) -> Range {
    let e = token.body_pos();
    let mut s = e;

    for t in token.leading.iter().rev() {
        match t.kind {
            TokenKind::Blank => {
                s = s.min(t.loc.range.start());
            }
            TokenKind::Newlines => {
                let last = t.text.rfind('\n').unwrap() + 1;
                s = s.min(t.loc.range.start() + Pos::from(&t.text[..last]));
                break;
            }
            _ => break,
        }
    }

    Range::from(s..e)
}

fn trailing_blank_range(token: &PToken) -> Range {
    let s = token.body.loc.end();
    let e = token
        .trailing
        .iter()
        .take_while(|t| t.kind == TokenKind::Blank)
        .map(|t| t.loc.range.end())
        .last()
        .unwrap_or(s);
    Range::from(s..e)
}

struct V {
    /// 地の文 (プリプロセッサ命令以外) の字下げ
    ground_depth: i32,

    text: RcStr,
    tokens: RcSlice<PToken>,

    edits: Vec<(Range, String)>,
}

impl V {
    fn insert_blank(&mut self, pos: Pos) {
        self.edits.push((Range::empty(pos), " ".into()));
    }

    fn replace(&mut self, range: Range, new_text: String) {
        if new_text.is_empty() {
            self.remove(range);
            return;
        }

        self.edits.push((range, new_text));
    }

    fn remove(&mut self, range: Range) {
        if !range.is_empty() {
            self.edits.push((range, "".into()));
        }
    }

    fn find_previous_token(&self, token: &PToken) -> Option<RcItem<PToken>> {
        let i = self
            .tokens
            .binary_search_by_key(&token.body_pos(), PToken::body_pos)
            .ok()?;
        self.tokens.item(i.saturating_sub(1))
    }

    #[cfg(unused)]
    fn find_next_token(&self, token: &PToken) -> Option<RcItem<PToken>> {
        let i = self
            .tokens
            .binary_search_by_key(token.body_pos(), PToken::body_pos)
            .ok()?;
        self.tokens.item(i + 1)
    }

    fn get_leading_blank_range(&self, token: &PToken) -> Range {
        // 同じ行の前方に別のトークンがある場合、このトークンの前方の空白は直前のトークンのtrailingに含まれている。
        let leaded_by_newlines = token.leading.iter().any(|t| t.kind == TokenKind::Newlines);
        if !leaded_by_newlines {
            if let Some(prev) = self.find_previous_token(&token) {
                return trailing_blank_range(&prev);
            }
        }

        leading_blank_range(token)
    }

    fn remove_leading_blank(&mut self, token: &PToken) {
        // コメントがある場合は触らない。
        if !leading_in_same_line_is_all_blank(token) {
            return;
        }

        let range = self.get_leading_blank_range(&token);
        self.remove(range);
    }

    fn remove_trailing_blank(&mut self, token: &PToken) {
        // コメントがある場合は触らない。
        if !trailing_is_all_blank(token) {
            return;
        }

        self.remove(trailing_blank_range(token));
    }

    fn require_leading_blank(&mut self, token: &PToken) {
        // コメントがある場合は触らない。
        if !leading_in_same_line_is_all_blank(token) {
            return;
        }

        let range = self.get_leading_blank_range(token);
        if range.is_empty() {
            self.insert_blank(range.end());
        }
    }

    fn require_trailing_blank(&mut self, token: &PToken) {
        let range = trailing_blank_range(token);
        if !range.is_empty() {
            return;
        }

        // トークンの後ろに何もなかったらスペースを挿入しない。
        if self.text[range.end().index as usize..]
            .chars()
            .next()
            .map_or(true, |c| c.is_whitespace())
        {
            return;
        }

        self.insert_blank(range.end());
    }

    fn require_blank_around(&mut self, token: &PToken) {
        self.require_leading_blank(token);
        self.require_trailing_blank(token);
    }

    fn reset_ground_indent(&mut self, token: &PToken) {
        if self.ground_depth <= 0 {
            return;
        }
        let depth = self.ground_depth as usize;

        // 同じ行の前方に他のトークンがある場合、字下げの調整はできない。
        let leaded_by_newlines = token.leading.iter().any(|t| t.kind == TokenKind::Newlines);
        if !leaded_by_newlines {
            return;
        }

        let range = leading_blank_range(token);

        // インデントを挿入する範囲にタブ文字以外のものが含まれていたら書き換えないでおく。
        let mut n = 0;
        for c in self.text[index_range(range)].chars() {
            if c == '\t' {
                n += 1;
            } else {
                return;
            }
        }

        if n != depth {
            self.replace(range, iter::repeat('\t').take(depth).collect::<String>());
        }
    }
}

impl PVisitor for V {
    fn on_label(&mut self, label: &PLabel) {
        if label.name_opt.is_some() {
            self.remove_trailing_blank(&label.star);
        }
    }

    fn on_args(&mut self, args: &[PArg]) {
        self.on_args_default(args);

        for arg in args.iter().rev().skip(1) {
            if let (Some(_), Some(comma)) = (&arg.expr_opt, &arg.comma_opt) {
                self.remove_leading_blank(comma);
                self.require_trailing_blank(comma);
            }
        }
    }

    fn on_compound(&mut self, compound: &PCompound) {
        self.on_compound_default(compound);

        if let PCompound::Paren(np) = compound {
            if let Some(right_paren) = &np.right_paren_opt {
                self.remove_trailing_blank(&np.left_paren);
                self.remove_leading_blank(right_paren);
            }
        }
    }

    fn on_expr(&mut self, expr: &PExpr) {
        self.on_expr_default(expr);

        match expr {
            PExpr::Paren(expr) => {
                if let (Some(_), Some(right_paren)) = (&expr.body_opt, &expr.right_paren_opt) {
                    self.remove_trailing_blank(&expr.left_paren);
                    self.remove_leading_blank(right_paren);
                }
            }
            PExpr::Prefix(expr) => {
                if expr.arg_opt.is_some() {
                    self.remove_trailing_blank(&expr.prefix);
                }
            }
            PExpr::Infix(expr) => {
                if expr.right_opt.is_some() {
                    self.require_blank_around(&expr.infix);
                }
            }
            _ => {}
        }
    }

    fn on_stmt(&mut self, stmt: &PStmt) {
        if let PStmt::DefFunc(_) | PStmt::Module(_) = stmt {
            self.ground_depth = 1;
        }

        self.on_stmt_default(stmt);

        match stmt {
            PStmt::Assign(stmt) => {
                self.reset_ground_indent(stmt.left.name());

                if let Some(op) = &stmt.op_opt {
                    match op.kind() {
                        TokenKind::PlusPlus | TokenKind::MinusMinus => {}
                        _ => self.require_blank_around(op),
                    }
                }
            }
            PStmt::Command(stmt) => {
                let (d1, d2) = delta(stmt.command.body_text()).unwrap_or((0, 0));
                self.ground_depth += d1;
                self.reset_ground_indent(&stmt.command);
                self.ground_depth += d2;

                if !stmt.args.is_empty() {
                    self.require_trailing_blank(&stmt.command);
                }
            }
            PStmt::Invoke(stmt) => {
                self.reset_ground_indent(stmt.left.name());
            }
            PStmt::If(stmt) => {
                self.reset_ground_indent(&stmt.command);
                self.require_trailing_blank(&stmt.command);
            }
            _ => {}
        }
    }

    fn on_block(&mut self, block: &PBlock) {
        self.on_stmts(&block.outer_stmts);

        if let (Some(left), Some(right)) = (&block.left_opt, &block.right_opt) {
            self.require_leading_blank(left);

            self.ground_depth += 1;
            self.on_stmts(&block.inner_stmts);
            self.ground_depth -= 1;

            self.reset_ground_indent(right);
            self.require_trailing_blank(right);
        }
    }
}

pub(crate) fn formatting(wa: &AnalysisRef<'_>, uri: Url, docs: &Docs) -> Option<Vec<TextEdit>> {
    let doc = docs.find_by_uri(&CanonicalUri::from_url(&uri))?;
    let DocSyntax { text, tokens, root } = wa.get_syntax(doc)?;

    let mut ctx = V {
        ground_depth: 1,
        text,
        tokens,
        edits: vec![],
    };
    ctx.on_root(root);

    let mut edits = ctx.edits;
    edits.sort_by_key(|(range, text)| (range.start(), text.len()));

    // 重なった変更を削除する。
    let mut last = Pos16::new(0, 0);
    edits.retain(|(range, _)| {
        let ok = last <= range.start();
        last = range.end().into();
        ok
    });

    let edits = edits
        .into_iter()
        .map(|(range, new_text)| TextEdit {
            range: to_lsp_range(range),
            new_text,
        })
        .collect::<Vec<_>>();

    Some(edits)
}
