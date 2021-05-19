//! カンマの両側を交換するアクション

use super::*;
use crate::{
    analysis::WorkspaceAnalysis, assists::from_document_position, lang_service::docs::Docs,
    parse::*,
};
use lsp_types::{
    CodeAction, CodeActionContext, DocumentChanges, Range, TextDocumentEdit, TextEdit, Url,
    VersionedTextDocumentIdentifier, WorkspaceEdit,
};

/// 構文木におけるトークンの深さを計算するためのビジター。
#[derive(Default)]
struct V {
    depths: HashMap<Pos16, usize>,
    depth: usize,
}

impl PVisitor for V {
    fn on_token(&mut self, token: &PToken) {
        let depth = self.depth;
        self.depths.insert(token.body_pos16(), depth);
    }

    fn on_compound(&mut self, compound: &PCompound) {
        self.depth += 1;
        self.on_compound_default(compound);
        self.depth -= 1;
    }

    fn on_expr(&mut self, expr: &PExpr) {
        self.depth += 1;
        self.on_expr_default(expr);
        self.depth -= 1;
    }

    fn on_args(&mut self, args: &[PArg]) {
        self.depth += 1;
        self.on_args_default(args);
        self.depth -= 1;
    }

    fn on_param(&mut self, param: &PParam) {
        self.depth += 2;
        if let Some((_, token)) = &param.param_ty_opt {
            self.on_token(token);
        }
        self.on_token_opt(param.name_opt.as_ref());
        self.depth -= 1;
        self.on_token_opt(param.comma_opt.as_ref());
        self.depth -= 1;
    }

    fn on_params(&mut self, params: &[PParam]) {
        self.depth += 1;
        self.on_params_default(params);
        self.depth -= 1;
    }

    fn on_stmt(&mut self, stmt: &PStmt) {
        self.depth += 1;
        self.on_stmt_default(stmt);
        self.depth -= 1;
    }
}

pub(crate) fn flip_comma(
    uri: Url,
    range: Range,
    _context: CodeActionContext,
    docs: &Docs,
    wa: &mut WorkspaceAnalysis,
) -> Option<Vec<CodeAction>> {
    let (doc, pos) = from_document_position(&uri, range.start, &docs)?;
    let version = docs.get_version(doc);

    let (text, tokens, root) = wa.get_tokens(doc)?;

    // 補完位置に隣接しているカンマをみつける。
    let (comma_index, comma) = {
        let i = match tokens.binary_search_by_key(&pos, PToken::body_pos16) {
            Ok(i) | Err(i) => i.saturating_sub(1),
        };
        let (offset, comma) = tokens[i..].iter().enumerate().take(3).find(|(_, t)| {
            t.kind() == TokenKind::Comma && t.body.loc.range.contains_inclusive(pos)
        })?;
        (i + offset, comma)
    };
    if comma_index == 0 || comma_index + 1 >= tokens.len() {
        return None;
    }

    // トークンの深さを計算する。
    let depths = {
        let mut v = V::default();
        v.depth = 1;
        v.on_root(root);
        v.depths
    };
    let d = |t: &PToken| depths.get(&t.body_pos16()).cloned().unwrap_or(0);

    // カンマの両脇の構文ノードに含まれるトークンをみつける。
    let comma_depth = d(comma);

    let left = tokens[..comma_index]
        .iter()
        .rev()
        .take_while(|t| comma_depth < d(t))
        .count();
    if left == 0 {
        return None;
    }

    let right = tokens[comma_index + 1..]
        .iter()
        .take_while(|t| comma_depth < d(t))
        .count();
    if right == 0 {
        return None;
    }

    // 両脇のノードの範囲と文字列:
    let l_range = source::Range::from(
        tokens[comma_index - left].body.loc.range.start()
            ..tokens[comma_index - 1].body.loc.range.end(),
    );
    let l_text = &text[l_range.start().index as usize..l_range.end().index as usize];

    let r_range = source::Range::from(
        comma.behind().range.end()..tokens[comma_index + right].body.loc.range.end(),
    );
    let r_text = &text[r_range.start().index as usize..r_range.end().index as usize];

    Some(vec![CodeAction {
        title: "カンマの両側を交換".into(),
        kind: Some("refactor.rewrite".into()),
        edit: Some(WorkspaceEdit {
            document_changes: Some(DocumentChanges::Edits(vec![TextDocumentEdit {
                text_document: VersionedTextDocumentIdentifier {
                    uri: uri.clone(),
                    version: version,
                },
                edits: vec![
                    TextEdit {
                        range: to_lsp_range(l_range),
                        new_text: r_text.to_string(),
                    },
                    TextEdit {
                        range: to_lsp_range(r_range),
                        new_text: l_text.to_string(),
                    },
                ],
            }])),
            ..WorkspaceEdit::default()
        }),
        ..Default::default()
    }])
}
