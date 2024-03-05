#[macro_use]
extern crate log;

pub mod c_api;

mod assists;
mod help_source;
mod lang;
mod lang_service;
mod lsp_server;
mod tests;

use token::{tokenize, TokenKind};

pub use crate::lsp_server::lsp_main::start_lsp_server;

/// 多くのモジュールからインポートされるシンボル:
use crate::utils::{
    canonical_uri::CanonicalUri, rc_item::RcItem, rc_slice::RcSlice, rc_str::RcStr,
};

#[allow(unused)]
use std::{
    cell::{Cell, RefCell},
    cmp::Ordering,
    collections::{HashMap, HashSet},
    fmt::{self, Debug, Display, Formatter},
    fs,
    hash::{Hash, Hasher},
    io, iter,
    marker::PhantomData,
    mem::{replace, take},
    ops::Deref,
    path::{self, Path, PathBuf},
    rc::Rc,
};

mod analysis {
    use super::*;

    mod comment;
    mod compute_active_docs;
    mod compute_symbols;
    mod doc_analysis;
    mod integrate;
    mod name_system;
    mod preproc;
    mod project_analysis;
    mod sema_linter;
    mod symbol;
    mod syntax_linter;
    mod var;
    mod workspace_analysis;

    pub(crate) use self::{
        doc_analysis::{in_preproc, in_str_or_comment, resolve_scope_at, DocAnalysis},
        name_system::*,
        preproc::{IncludeGuard, PreprocAnalysisResult, SignatureData},
        project_analysis::{EntryPoints, ProjectAnalysis, ProjectAnalysisRef},
        sema_linter::{Diagnostic, SemaLinter},
        symbol::{
            module_name_as_ident, DefFuncData, DefFuncKey, DefFuncMap, ModuleData, ModuleKey,
            ModuleMap, ModuleRc,
        },
        symbol::{DefInfo, HspSymbolKind, SymbolDetails, SymbolRc},
        syntax_linter::SyntaxLint,
        workspace_analysis::{
            collect_doc_symbols, collect_preproc_completion_items, collect_symbols_in_scope,
            collect_workspace_symbols, find_include_target, DocSyntax, WorkspaceAnalysis,
            WorkspaceHost,
        },
    };

    use crate::{
        lang::Lang,
        lang_service::{docs::ProjectDocs, search_hsphelp::HspHelpInfo},
        parse::{PRoot, PToken},
        source::*,
        token::{TokenData, TokenKind},
    };
}

mod parse {
    //! 構文木・構文解析

    pub(crate) mod p_const_ty;
    pub(crate) mod p_jump_modifier;
    pub(crate) mod p_op_kind;
    pub(crate) mod p_param_ty;
    pub(crate) mod p_privacy;
    pub(crate) mod p_token;
    pub(crate) mod p_tree;
    pub(crate) mod p_visitor;
    pub(crate) mod parse_context;
    pub(crate) mod parse_expr;
    pub(crate) mod parse_preproc;
    pub(crate) mod parse_stmt;

    mod parse_tests;

    pub(crate) use p_const_ty::PConstTy;
    pub(crate) use p_jump_modifier::PJumpModifier;
    pub(crate) use p_param_ty::PParamTy;
    pub(crate) use p_privacy::PPrivacy;
    pub(crate) use p_token::PToken;
    pub(crate) use p_tree::*;
    pub(crate) use p_visitor::PVisitor;

    pub(crate) use parse_stmt::parse_root;

    use self::parse_context::Px;
    use super::*;
    use crate::{
        source::*,
        token::{TokenData, TokenKind},
    };
}

mod project_model {
    //! プロジェクトモデル
    // (ディレクトリ内のファイル間の関係について扱うモジュール)

    pub(crate) mod scan;
}

mod source {
    //! ソースファイルの位置情報など

    mod loc;

    pub(crate) use loc::*;

    pub(crate) type DocId = usize;
    pub(crate) type Pos = text_position_rs::CompositePosition;
    pub(crate) type Pos16 = text_position_rs::Utf16Position;
    pub(crate) type Range = text_position_rs::TextRange<Pos>;
    pub(crate) type Range16 = text_position_rs::TextRange<Pos16>;

    pub(crate) fn range_is_touched(range: &Range, pos: Pos16) -> bool {
        Range16::from(Pos16::from(range.start())..Pos16::from(range.end())).contains_inclusive(pos)
    }
}

mod token {
    //! 字句・字句解析

    mod token_data;
    mod token_kind;
    mod tokenize_context;
    mod tokenize_rules;
    mod tokenize_tests;

    pub(crate) use token_data::TokenData;
    pub(crate) use token_kind::TokenKind;
    pub(crate) use tokenize_rules::tokenize;

    use super::*;
    use crate::source::*;
}

mod utils {
    pub(crate) mod canonical_uri;
    pub(crate) mod rc_item;
    pub(crate) mod rc_slice;
    pub(crate) mod rc_str;
    pub(crate) mod read_file;
}

pub fn rewrite_fn(text: String) -> String {
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
