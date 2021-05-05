use crate::{
    analysis::{
        comment::{calculate_details, collect_comments},
        integrate::{ACompletionItem, AWorkspaceAnalysis},
        AScope, ASymbolKind,
    },
    assists::from_document_position,
    lang_service::docs::Docs,
    parse::p_param_ty::PParamCategory,
    source::*,
    token::TokenKind,
};
use lsp_types::{CompletionItem, CompletionItemKind, CompletionList, Documentation, Position, Url};

fn is_preproc_statement(doc: DocId, pos: Pos16, wa: &AWorkspaceAnalysis) -> bool {
    let tokens = match wa.doc_syntax_map.get(&doc) {
        Some(syntax) => &syntax.tokens,
        None => return false,
    };

    // '#' から文末の間においてプリプロセッサ関連の補完を有効化する。行継続に注意。判定が難しいので構文木を使ったほうがいいかもしれない。

    let row = pos.row as usize;

    // 次の行の最初のトークンを探す。
    let upperbound =
        match tokens.binary_search_by_key(&(row + 1), |token| token.body.loc.start_row()) {
            Ok(it) | Err(it) => it,
        };

    // 近くにあるトークンと補完位置の位置関係を調べる。
    // (補完位置の付近にトークンがないとき、次の '#' の検索だけだとプリプロセッサ行の後ろがすべて引っかかってしまう。)
    let last = tokens.get(upperbound.saturating_sub(1));
    let touched = last.map_or(false, |t| pos <= t.behind().end());

    // 補完位置から遡って '#' を探す。同じ文の中で、補完位置より手前にあったらOK。
    let hash_found = touched
        && tokens[..upperbound]
            .iter()
            .rev()
            .skip(1)
            .take_while(|token| token.kind() != TokenKind::Eos)
            .any(|token| {
                token.kind() == TokenKind::Hash
                    && Pos16::from(token.body.loc.ahead().start()) <= pos
            });
    hash_found
}

fn collect_preproc_completion_items(
    other_items: &[CompletionItem],
    items: &mut Vec<CompletionItem>,
) {
    let sort_prefix = 'a';

    for (keyword, detail) in &[
        ("ctype", "関数形式のマクロを表す"),
        ("global", "グローバルスコープを表す"),
        ("local", "localパラメータ、またはローカルスコープを表す"),
        ("int", "整数型のパラメータ、または整数型の定数を表す"),
        ("double", "実数型のパラメータ、または実数型の定数を表す"),
        ("str", "文字列型のパラメータを表す"),
        ("label", "ラベル型のパラメータを表す"),
        ("var", "変数 (配列要素) のパラメータを表す"),
        ("array", "配列変数のパラメータを表す"),
    ] {
        items.push(CompletionItem {
            kind: Some(CompletionItemKind::Keyword),
            label: keyword.to_string(),
            detail: Some(detail.to_string()),
            sort_text: Some(format!("{}{}", sort_prefix, keyword)),
            ..CompletionItem::default()
        });
    }

    items.extend(
        other_items
            .iter()
            .filter(|item| item.label.starts_with("#"))
            .cloned(),
    );
}

pub(crate) fn incomplete_completion_list() -> CompletionList {
    CompletionList {
        is_incomplete: true,
        items: vec![],
    }
}

pub(crate) fn completion(
    uri: Url,
    position: Position,
    docs: &Docs,
    wa: &mut AWorkspaceAnalysis,
    other_items: &[CompletionItem],
) -> Option<CompletionList> {
    let mut items = vec![];

    let (doc, pos) = from_document_position(&uri, position, docs)?;

    if wa.in_str_or_comment(doc, pos).unwrap_or(true) {
        return None;
    }

    if is_preproc_statement(doc, pos, wa) {
        collect_preproc_completion_items(other_items, &mut items);
        return Some(CompletionList {
            is_incomplete: false,
            items,
        });
    }

    for item in wa.collect_completion_items(doc, pos) {
        match item {
            ACompletionItem::Symbol(symbol) => {
                let details = calculate_details(&collect_comments(&symbol.leader));

                use CompletionItemKind as K;

                let kind = match symbol.kind {
                    ASymbolKind::Unresolved => K::Text,
                    ASymbolKind::Label => K::Value,
                    ASymbolKind::StaticVar => K::Variable,
                    ASymbolKind::Const => K::Constant,
                    ASymbolKind::Enum => K::EnumMember,
                    ASymbolKind::Macro { ctype: false } => K::Value,
                    ASymbolKind::Macro { ctype: true } => K::Function,
                    ASymbolKind::DefFunc => K::Method,
                    ASymbolKind::DefCFunc => K::Function,
                    ASymbolKind::ModFunc => K::Method,
                    ASymbolKind::ModCFunc => K::Function,
                    ASymbolKind::Param(None) => K::Variable,
                    ASymbolKind::Param(Some(param)) => match param.category() {
                        PParamCategory::ByValue => K::Value,
                        PParamCategory::ByRef => K::Property,
                        PParamCategory::Local => K::Variable,
                        PParamCategory::Auto => K::Text,
                    },
                    ASymbolKind::Module => K::Module,
                    ASymbolKind::Field => K::Field,
                    ASymbolKind::LibFunc => K::Function,
                    ASymbolKind::PluginCmd => K::Keyword,
                    ASymbolKind::ComInterface => K::Interface,
                    ASymbolKind::ComFunc => K::Method,
                };

                // 候補の順番を制御するための文字。(スコープが狭いものを上に出す。)
                let sort_prefix = match (&symbol.scope_opt, symbol.kind) {
                    (Some(AScope::Local(local)), _) => match (&local.module_opt, local.deffunc_opt)
                    {
                        (Some(_), Some(_)) => 'a',
                        (Some(_), None) => 'b',
                        (None, None) => 'c',
                        (None, Some(_)) => 'd',
                    },
                    (_, ASymbolKind::Module) => 'f',
                    (Some(AScope::Global), _) => 'e',
                    (None, _) => 'g',
                };

                items.push(CompletionItem {
                    kind: Some(kind),
                    label: symbol.name.to_string(),
                    detail: details.desc.map(|s| s.to_string()),
                    documentation: if details.docs.is_empty() {
                        None
                    } else {
                        Some(Documentation::String(details.docs.join("\r\n\r\n")))
                    },
                    sort_text: Some(format!("{}{}", sort_prefix, symbol.name)),
                    ..CompletionItem::default()
                });
            }
        }
    }

    items.extend(
        other_items
            .iter()
            .filter(|item| !item.label.starts_with("#"))
            .cloned(),
    );

    Some(CompletionList {
        is_incomplete: false,
        items,
    })
}
