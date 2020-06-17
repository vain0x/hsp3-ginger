//! FIXME: func, cfunc, cmd

use crate::{
    analysis::{ADoc, ALoc, APos},
    token::{TokenData, TokenKind},
    utils::rc_str::RcStr,
};
use std::{collections::HashMap, rc::Rc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SymbolKind {
    Static,
    Label,
    Macro {
        global: bool,
        ctype: bool,
    },
    Command {
        local: bool,
        ctype: bool,
    },
    Param {
        command_start: Option<usize>,
        command_end: Option<usize>,
    },
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Scope {
    pub(crate) doc: ADoc,
    pub(crate) row_range: (usize, usize),
    pub(crate) is_global: bool,
    pub(crate) only_global: bool,
}

impl Scope {
    pub(crate) fn new_global(doc: ADoc) -> Self {
        Self {
            doc,
            row_range: (0, std::usize::MAX),
            is_global: true,
            only_global: false,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SymbolDetails {
    pub(crate) description: Option<RcStr>,
    pub(crate) documentation: Vec<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct Symbol {
    pub(crate) symbol_id: usize,
    pub(crate) name: RcStr,
    pub(crate) details: SymbolDetails,
    pub(crate) kind: SymbolKind,
    pub(crate) scope: Scope,
}

type SymbolMap = HashMap<RcStr, Vec<Rc<Symbol>>>;

// 行の種類。
// 複数行文字列や複数行コメントに分類された行は後続の処理に渡されないので、そのための種類は必要ない。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum LineKind {
    Ground,
    PreProc,
}

#[derive(Clone, Debug)]
pub(crate) struct Line {
    pub(crate) kind: LineKind,
    pub(crate) doc: ADoc,
    pub(crate) row: usize,
    pub(crate) leading: Vec<RcStr>,
    pub(crate) words: Vec<(APos, APos, RcStr)>,
    pub(crate) module_start: Option<usize>,
    pub(crate) module_end: Option<usize>,
    pub(crate) command_start: Option<usize>,
    pub(crate) command_end: Option<usize>,
}

// ドキュメントの解析結果
pub(crate) struct DocSem {
    pub(crate) doc: ADoc,
    #[allow(unused)]
    pub(crate) text: RcStr,
    pub(crate) lines: Vec<Line>,
    #[allow(unused)]
    pub(crate) line_count: usize,
    #[allow(unused)]
    pub(crate) module_ranges: HashMap<usize, usize>,
    #[allow(unused)]
    pub(crate) command_ranges: HashMap<usize, usize>,
    pub(crate) pp_symbols: HashMap<usize, Rc<Symbol>>,
    pub(crate) pp_symbol_defs: HashMap<usize, Vec<ALoc>>,
}

// プロジェクト全体の解析結果
#[derive(Default)]
pub(crate) struct ProjectSem {
    pub(crate) docs: HashMap<ADoc, DocSem>,
    pub(crate) last_symbol_id: usize,
    pub(crate) all_symbols: HashMap<usize, Rc<Symbol>>,
    pub(crate) all_symbol_defs: HashMap<usize, Vec<ALoc>>,
    pub(crate) all_symbol_uses: HashMap<usize, Vec<ALoc>>,
    pub(crate) all_symbol_map: SymbolMap,
    pub(crate) is_dirty: bool,
}

impl Line {
    fn is_module_decl(&self) -> bool {
        self.words.get(1).map(|t| &t.2 == "module").unwrap_or(false)
    }

    fn is_global_decl(&self) -> bool {
        self.words.get(1).map(|t| &t.2 == "global").unwrap_or(false)
    }

    fn is_cfunc_decl(&self) -> bool {
        self.words.get(1).map(|t| &t.2 == "cfunc").unwrap_or(false)
    }

    fn is_command_decl(&self) -> bool {
        self.words
            .get(1)
            .map(|t| ["deffunc", "defcfunc", "modfunc", "modcfunc"].contains(&t.2.as_str()))
            .unwrap_or(false)
    }

    fn is_macro_decl(&self) -> bool {
        self.words
            .get(1)
            .map(|t| ["const", "define", "enum"].contains(&t.2.as_str()))
            .unwrap_or(false)
    }
}

impl TokenKind {
    fn is_leading_trivial(self) -> bool {
        match self {
            TokenKind::Eol | TokenKind::Space | TokenKind::Comment | TokenKind::Other => true,
            _ => false,
        }
    }

    fn is_trailing_trivial(self) -> bool {
        match self {
            TokenKind::Space | TokenKind::Comment | TokenKind::Other => true,
            _ => false,
        }
    }

    /// 文の終わりを表す字句か？
    ///
    /// プリプロセッサ行では改行で文が終わる。(エスケープされた改行は字句解析の段階で処理している。)
    /// 通常の行では改行に加えて : や {} も文の終わりとみなす。
    fn is_terminator(self, preproc: bool) -> bool {
        match (self, preproc) {
            (TokenKind::Eos, _)
            | (TokenKind::Colon, false)
            | (TokenKind::LeftBrace, false)
            | (TokenKind::RightBrace, false) => true,
            _ => false,
        }
    }
}

/// トリビアでないトークンに前後のトリビアをくっつけたものをPトークンと呼ぶ。
#[derive(Clone, Debug)]
struct PToken {
    leading: Vec<TokenData>,
    body: TokenData,
    trailing: Vec<TokenData>,
}

impl PToken {
    pub(crate) fn kind(&self) -> TokenKind {
        self.body.kind
    }

    fn behind(&self) -> ALoc {
        match self.trailing.last() {
            Some(last) => last.loc.behind(),
            None => self.body.loc.behind(),
        }
    }
}

fn convert_tokens(tokens: Vec<TokenData>) -> Vec<PToken> {
    let empty_text = {
        let eof = tokens.last().unwrap();
        eof.text.slice(0, 0)
    };

    // 空白やコメントなど、構文上の役割を持たないトークンをトリビアと呼ぶ。
    // トリビアは解析の邪魔なので、トリビアでないトークンの前後にくっつける。
    let mut tokens = tokens.into_iter().peekable();
    let mut p_tokens = vec![];
    let mut leading = vec![];
    let mut trailing = vec![];

    loop {
        // トークンの前にあるトリビアは先行トリビアとする。
        while tokens.peek().map_or(false, |t| t.kind.is_leading_trivial()) {
            leading.push(tokens.next().unwrap());
        }

        let body = match tokens.next() {
            Some(body) => {
                assert!(!body.kind.is_leading_trivial());
                body
            }
            None => break,
        };

        while tokens
            .peek()
            .map_or(false, |t| t.kind.is_trailing_trivial())
        {
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

fn make_lines(doc: ADoc, tokens: Vec<PToken>, lines: &mut Vec<Line>) {
    let mut tokens = tokens.into_iter().peekable();
    let mut leading = vec![];
    let mut words = vec![];

    while let Some(token) = tokens.next() {
        let preproc = match token.kind() {
            TokenKind::Eof => break,
            TokenKind::Hash => true,
            _ => false,
        };

        let row = token.body.loc.start_row();

        leading.extend(token.leading.into_iter().filter_map(|token| {
            if token.kind == TokenKind::Comment {
                Some(token.text)
            } else {
                None
            }
        }));

        let body = token.body;
        words.push((body.loc.start(), body.loc.end(), body.text));

        loop {
            if tokens
                .peek()
                .map_or(true, |t| t.body.kind.is_terminator(preproc))
            {
                break;
            }

            let body = tokens.next().unwrap().body;
            words.push((body.loc.start(), body.loc.end(), body.text));
        }

        while tokens
            .peek()
            .map_or(false, |t| t.body.kind.is_terminator(preproc))
        {
            tokens.next();
        }

        let kind = if preproc {
            LineKind::PreProc
        } else {
            LineKind::Ground
        };
        lines.push(Line {
            kind,
            doc,
            row,
            leading: leading.split_off(0),
            words: words.split_off(0),
            module_start: None,
            module_end: None,
            command_start: None,
            command_end: None,
        });
    }
}

pub(crate) fn tokenize(doc: ADoc, text: RcStr, lines: &mut Vec<Line>, line_count: &mut usize) {
    let tokens = crate::token::tokenize(doc, text);
    *line_count = tokens
        .last()
        .as_ref()
        .map_or(0, |token| token.loc.end_row());
    let tokens = convert_tokens(tokens);
    make_lines(doc, tokens, lines)
}

// module/deffunc の範囲を計算する。
pub(crate) fn analyze_pp_scopes(
    line_count: usize,
    lines: &mut Vec<Line>,
    module_ranges: &mut HashMap<usize, usize>,
    command_ranges: &mut HashMap<usize, usize>,
) {
    let mut module_start = None;
    let mut command_start = None;

    for line in lines.iter_mut() {
        if line.kind == LineKind::PreProc {
            if line.is_module_decl() {
                module_start = Some(line.row);
            }
            if line.is_global_decl() {
                if let Some(module_start) = module_start {
                    module_ranges.insert(module_start, line.row);
                }
                if let Some(command_start) = command_start {
                    command_ranges.insert(command_start, line.row);
                }
                module_start = None;
                command_start = None;
            }
            if line.is_command_decl() {
                if let Some(command_start) = command_start {
                    command_ranges.insert(command_start, line.row);
                }
                command_start = Some(line.row);
            }
        }

        line.module_start = module_start;
        line.command_start = command_start;
    }
    if let Some(module_start) = module_start {
        module_ranges.insert(module_start, line_count);
    }
    if let Some(command_start) = command_start {
        command_ranges.insert(command_start, line_count);
    }

    for line in lines.iter_mut() {
        let module_end = line
            .module_start
            .and_then(|row| module_ranges.get(&row).cloned());
        line.module_end = module_end;

        let command_end = line
            .command_start
            .and_then(|row| command_ranges.get(&row).cloned());
        line.command_end = command_end;
    }
}

fn calculate_details(lines: &[RcStr]) -> SymbolDetails {
    fn char_is_ornament_comment(c: char) -> bool {
        c.is_whitespace() || c == ';' || c == '/' || c == '*' || c == '-' || c == '=' || c == '#'
    }

    // 装飾コメント (// ---- とか) か空行
    fn str_is_ornament_comment(s: &str) -> bool {
        s.chars().all(char_is_ornament_comment)
    }

    let mut description = None;
    let mut documentation = vec![];

    let mut y = 0;

    for line in lines {
        y += 1;

        // 装飾コメントや空行を無視
        let t = line.as_str().trim();
        if str_is_ornament_comment(t) {
            continue;
        }

        // 最初の行は概要
        description = Some(line.clone());
        break;
    }

    for line in &lines[y..] {
        // 装飾コメントや空行を無視
        let t = line.as_str().trim();
        if str_is_ornament_comment(t) {
            y += 1;
            continue;
        }
        break;
    }

    // 残りの行はドキュメンテーション
    if y < lines.len() {
        documentation.push(
            lines[y..]
                .into_iter()
                .map(|s| s.as_str().trim())
                .collect::<Vec<_>>()
                .join("\r\n"),
        );
    }

    SymbolDetails {
        description,
        documentation,
    }
}

fn calculate_scope(
    doc: ADoc,
    kind: SymbolKind,
    module_start: Option<usize>,
    module_end: Option<usize>,
) -> Scope {
    let is_global = match kind {
        SymbolKind::Static | SymbolKind::Label => module_start.is_none(),
        SymbolKind::Command { local: false, .. } | SymbolKind::Macro { global: true, .. } => true,
        _ => false,
    };

    let is_local = !is_global;

    let only_global = is_global
        && match kind {
            SymbolKind::Static | SymbolKind::Label => true,
            _ => false,
        };

    let row_range = match kind {
        SymbolKind::Param {
            command_start,
            command_end,
            ..
        } => {
            let start = command_start.unwrap_or(0);
            let end = command_end.unwrap_or(std::usize::MAX);
            (start, end)
        }
        _ if is_local => {
            let start = module_start.unwrap_or(0);
            let end = module_end.unwrap_or(std::usize::MAX);
            (start, end)
        }
        _ => (0, std::usize::MAX),
    };

    Scope {
        doc,
        row_range,
        is_global,
        only_global,
    }
}

// deffunc etc.
pub(crate) fn collect_commands(
    doc: ADoc,
    lines: &mut Vec<Line>,
    last_symbol_id: &mut usize,
    symbols: &mut HashMap<usize, Rc<Symbol>>,
    symbol_defs: &mut HashMap<usize, Vec<ALoc>>,
) {
    for line in lines.iter_mut() {
        if !(line.kind == LineKind::PreProc && line.is_command_decl() && line.words.len() >= 2) {
            continue;
        }

        let mut first = true;
        let mut global = false;
        let mut local = false;
        let ctype = line.is_cfunc_decl();

        for &(start, end, ref word) in &line.words[2..] {
            let is_keyword = [
                "global", "local", "int", "str", "double", "label", "var", "array", "modvar",
                "onexit",
            ]
            .contains(&word.as_str());

            if is_keyword {
                global = global || word.as_str() == "global";
                local = local || (first && word.as_str() == "local");
                continue;
            }

            let kind = if first {
                first = false;
                SymbolKind::Command { local, ctype }
            } else {
                SymbolKind::Param {
                    command_start: line.command_start,
                    command_end: line.command_end,
                }
            };

            let name = word.clone();
            let loc = ALoc::new3(doc, start, end);
            let symbol_id = {
                *last_symbol_id += 1;
                *last_symbol_id
            };
            symbols.insert(
                symbol_id,
                Rc::new(Symbol {
                    symbol_id,
                    name,
                    kind,
                    details: calculate_details(&line.leading),
                    scope: calculate_scope(doc, kind, line.module_start, line.module_end),
                }),
            );
            symbol_defs.insert(symbol_id, vec![loc]);
        }
    }
}

// define/enum/const
pub(crate) fn collect_macro(
    doc: ADoc,
    lines: &mut Vec<Line>,
    last_symbol_id: &mut usize,
    symbols: &mut HashMap<usize, Rc<Symbol>>,
    symbol_defs: &mut HashMap<usize, Vec<ALoc>>,
) {
    for line in lines.iter_mut() {
        if !(line.kind == LineKind::PreProc && line.is_macro_decl() && line.words.len() >= 2) {
            continue;
        }

        let mut global = false;
        let mut ctype = false;

        for &(start, end, ref word) in &line.words[2..] {
            let is_keyword = ["global", "local", "ctype", "double", "int"].contains(&word.as_str());

            if is_keyword {
                global = global || word.as_str() == "global";
                ctype = ctype || word.as_str() == "ctype";
                continue;
            }

            let name = word.clone();
            let kind = SymbolKind::Macro { global, ctype };
            let loc = ALoc::new3(doc, start, end);
            let symbol_id = {
                *last_symbol_id += 1;
                *last_symbol_id
            };
            symbols.insert(
                symbol_id,
                Rc::new(Symbol {
                    symbol_id,
                    name,
                    kind,
                    details: calculate_details(&line.leading),
                    scope: calculate_scope(doc, kind, line.module_start, line.module_end),
                }),
            );
            symbol_defs.insert(symbol_id, vec![loc]);
        }
    }
}

pub(crate) fn collect_labels(
    doc: ADoc,
    lines: &mut Vec<Line>,
    last_symbol_id: &mut usize,
    symbols: &mut HashMap<usize, Rc<Symbol>>,
    symbol_defs: &mut HashMap<usize, Vec<ALoc>>,
) {
    for line in lines.iter_mut() {
        if !(line.kind == LineKind::Ground
            && line.words.len() >= 2
            && line.words[0].2.as_str() == "*")
        {
            continue;
        }

        let (start, end, name) = line.words[1].clone();
        let kind = SymbolKind::Label;
        let loc = ALoc::new3(doc, start, end);
        let symbol_id = {
            *last_symbol_id += 1;
            *last_symbol_id
        };
        symbols.insert(
            symbol_id,
            Rc::new(Symbol {
                symbol_id,
                name,
                kind,
                details: calculate_details(&line.leading),
                scope: calculate_scope(doc, kind, line.module_start, line.module_end),
            }),
        );
        symbol_defs.insert(symbol_id, vec![loc]);
    }
}

// 静的変数をみつける。
// dim などの第一引数になっているか、単純な代入文 (x = ...)　の左辺に来ているものだけを候補とする。

pub(crate) fn collect_static_vars(
    doc: ADoc,
    lines: &mut Vec<Line>,
    last_symbol_id: &mut usize,
    symbols: &mut HashMap<usize, Rc<Symbol>>,
    symbol_defs: &mut HashMap<usize, Vec<ALoc>>,
    symbol_map: &mut SymbolMap,
) {
    static KEYWORDS: &[&str] = &[
        "dim", "sdim", "ddim", "ldim", "dimtype", "newmod", "newlab", "dup", "dupptr",
    ];

    for line in lines.iter() {
        if !(line.kind == LineKind::Ground && line.words.len() >= 2) {
            continue;
        }

        let (start, end, name) = if KEYWORDS.contains(&line.words[0].2.as_str()) {
            line.words[1].clone()
        } else if line.words[1].2.as_str() == "=" {
            line.words[0].clone()
        } else {
            continue;
        };

        if symbol_map
            .get(name.as_str())
            .unwrap_or(&vec![])
            .iter()
            .any(|symbol| symbol_is_in_scope(symbol, doc, line.row))
        {
            continue;
        }

        let loc = ALoc::new3(doc, start, end);
        let kind = SymbolKind::Static;
        let symbol_id = {
            *last_symbol_id += 1;
            *last_symbol_id
        };
        let symbol = Rc::new(Symbol {
            symbol_id,
            name: name.clone(),
            kind,
            details: calculate_details(&line.leading),
            scope: calculate_scope(doc, kind, line.module_start, line.module_end),
        });
        symbols.insert(symbol_id, symbol.clone());
        symbol_defs.insert(symbol_id, vec![loc]);
        symbol_map.entry(name).or_insert(vec![]).push(symbol);
    }
}

fn symbol_is_in_scope(symbol: &Symbol, doc: ADoc, row: usize) -> bool {
    if symbol.scope.is_global {
        // FIXME: check only_global
        true
    } else {
        symbol.scope.doc == doc && symbol.scope.row_range.0 <= row && row < symbol.scope.row_range.1
    }
}

pub(crate) fn resolve_uses(
    doc: ADoc,
    lines: &mut Vec<Line>,
    symbol_defs: &mut HashMap<usize, Vec<ALoc>>,
    symbol_uses: &mut HashMap<usize, Vec<ALoc>>,
    symbol_map: &SymbolMap,
) {
    for line in lines.iter() {
        for &(start, end, ref word) in &line.words {
            let symbol = match symbol_map
                .get(word.as_str())
                .into_iter()
                .flatten()
                .filter(|symbol| symbol_is_in_scope(symbol, doc, line.row))
                .next()
            {
                None => continue,
                Some(x) => x,
            };

            // 定義は使用に含めない
            if symbol_defs.get(&symbol.symbol_id).map_or(false, |locs| {
                locs.iter()
                    .any(|loc| loc.doc == doc && loc.start_row() == line.row)
            }) {
                continue;
            }

            symbol_uses
                .entry(symbol.symbol_id)
                .or_insert(vec![])
                .push(ALoc::new3(doc, start, end));
        }
    }
}

pub(crate) fn analyze_doc(doc: ADoc, text: RcStr, last_symbol_id: &mut usize) -> DocSem {
    let mut lines = vec![];
    let mut line_count = 0;
    let mut module_ranges = HashMap::new();
    let mut command_ranges = HashMap::new();
    let mut pp_symbols = HashMap::new();
    let mut pp_symbol_defs = HashMap::new();

    tokenize(doc, text.clone(), &mut lines, &mut line_count);
    analyze_pp_scopes(
        line_count,
        &mut lines,
        &mut module_ranges,
        &mut command_ranges,
    );
    collect_commands(
        doc,
        &mut lines,
        last_symbol_id,
        &mut pp_symbols,
        &mut pp_symbol_defs,
    );
    collect_macro(
        doc,
        &mut lines,
        last_symbol_id,
        &mut pp_symbols,
        &mut pp_symbol_defs,
    );
    collect_labels(
        doc,
        &mut lines,
        last_symbol_id,
        &mut pp_symbols,
        &mut pp_symbol_defs,
    );

    DocSem {
        doc,
        text,
        lines,
        line_count,
        module_ranges,
        command_ranges,
        pp_symbols,
        pp_symbol_defs,
    }
}

pub(crate) fn analyze_project(
    docs: &mut HashMap<ADoc, DocSem>,
    last_symbol_id: &mut usize,
    symbols: &mut HashMap<usize, Rc<Symbol>>,
    symbol_defs: &mut HashMap<usize, Vec<ALoc>>,
    symbol_uses: &mut HashMap<usize, Vec<ALoc>>,
    symbol_map: &mut SymbolMap,
) {
    for doc_sem in docs.values() {
        symbols.extend(
            doc_sem
                .pp_symbols
                .iter()
                .map(|(&symbol_id, symbol)| (symbol_id, symbol.clone())),
        );

        symbol_defs.extend(
            doc_sem
                .pp_symbol_defs
                .iter()
                .map(|(&symbol_id, locs)| (symbol_id, locs.clone())),
        );
    }

    for symbol in symbols.values() {
        symbol_map
            .entry(symbol.name.clone())
            .or_insert(vec![])
            .push(symbol.clone());
    }

    for doc_sem in docs.values_mut() {
        collect_static_vars(
            doc_sem.doc,
            &mut doc_sem.lines,
            last_symbol_id,
            symbols,
            symbol_defs,
            symbol_map,
        );
    }

    for doc_sem in docs.values_mut() {
        resolve_uses(
            doc_sem.doc,
            &mut doc_sem.lines,
            symbol_defs,
            symbol_uses,
            &symbol_map,
        );
    }
}

impl ProjectSem {
    pub(crate) fn new() -> Self {
        ProjectSem::default()
    }

    pub(crate) fn add_hs_symbols(&mut self, doc: ADoc, hs_symbols: Vec<Rc<Symbol>>) {
        self.is_dirty = true;
        self.docs.insert(
            doc,
            DocSem {
                doc,
                text: "".to_string().into(),
                lines: vec![],
                line_count: 0,
                module_ranges: HashMap::new(),
                command_ranges: HashMap::new(),
                pp_symbols: hs_symbols
                    .into_iter()
                    .map(|symbol| (symbol.symbol_id, symbol))
                    .collect(),
                pp_symbol_defs: HashMap::new(),
            },
        );
    }

    pub(crate) fn update_doc(&mut self, doc: ADoc, text: RcStr) {
        self.is_dirty = true;
        self.docs.remove(&doc);

        let doc_sem = analyze_doc(doc, text, &mut self.last_symbol_id);
        self.docs.insert(doc, doc_sem);
    }

    pub(crate) fn close_doc(&mut self, doc: ADoc) {
        self.is_dirty = true;
        self.docs.remove(&doc);
    }

    pub(crate) fn compute(&mut self) {
        if !std::mem::replace(&mut self.is_dirty, false) {
            return;
        }

        let mut symbols = std::mem::replace(&mut self.all_symbols, HashMap::new());
        symbols.clear();
        let mut symbol_defs = std::mem::replace(&mut self.all_symbol_defs, HashMap::new());
        symbol_defs.clear();
        let mut symbol_uses = std::mem::replace(&mut self.all_symbol_uses, HashMap::new());
        symbol_uses.clear();
        let mut symbol_map = std::mem::replace(&mut self.all_symbol_map, HashMap::new());
        symbol_map.clear();

        let mut last_symbol_id = self.last_symbol_id;
        analyze_project(
            &mut self.docs,
            &mut last_symbol_id,
            &mut symbols,
            &mut symbol_defs,
            &mut symbol_uses,
            &mut symbol_map,
        );

        self.all_symbols = symbols;
        self.all_symbol_defs = symbol_defs;
        self.all_symbol_uses = symbol_uses;
        self.all_symbol_map = symbol_map;
        self.last_symbol_id = last_symbol_id;
    }

    pub(crate) fn get_symbol_list(&mut self, doc: ADoc, pos: APos, symbols: &mut Vec<Rc<Symbol>>) {
        self.compute();

        symbols.extend(
            self.all_symbols
                .values()
                .filter(|symbol| symbol_is_in_scope(symbol, doc, pos.row))
                .cloned(),
        )
    }

    pub(crate) fn locate_symbol(&mut self, doc: ADoc, pos: APos) -> Option<(&Symbol, ALoc)> {
        self.compute();

        for (&symbol_id, locs) in self
            .all_symbol_defs
            .iter()
            .chain(self.all_symbol_uses.iter())
        {
            for &loc in locs.iter() {
                if loc.is_touched(doc, pos) {
                    if let Some(symbol) = self.all_symbols.get(&symbol_id) {
                        return Some((symbol, loc));
                    }
                }
            }
        }

        None
    }

    pub(crate) fn get_symbol_defs(&mut self, symbol_id: usize, locs: &mut Vec<ALoc>) {
        self.compute();

        locs.extend(self.all_symbol_defs.get(&symbol_id).into_iter().flatten());
    }

    pub(crate) fn get_symbol_uses(&mut self, symbol_id: usize, locs: &mut Vec<ALoc>) {
        self.compute();

        locs.extend(self.all_symbol_uses.get(&symbol_id).into_iter().flatten());
    }
}

#[cfg(test)]
mod tests {
    use super::{tokenize, LineKind};
    use crate::analysis::ADoc;

    fn to_strings(words: &[&str]) -> Vec<String> {
        words.iter().copied().map(Into::into).collect()
    }

    fn analyze(source_code: &str) -> Vec<(LineKind, Vec<String>)> {
        let doc = ADoc::new(1);
        let mut lines = vec![];
        let mut line_count = 0;

        tokenize(
            doc,
            source_code.to_string().into(),
            &mut lines,
            &mut line_count,
        );

        lines
            .into_iter()
            .map(|line| {
                (
                    line.kind,
                    line.words
                        .into_iter()
                        .map(|word| word.2.to_string())
                        .collect(),
                )
            })
            .collect()
    }

    #[test]
    fn preproc_statement_is_not_separated_by_colon_or_braces() {
        let text = r#"
            #define lnln if 0 { mes : mes }
        "#;
        assert_eq!(
            analyze(text),
            vec![(
                LineKind::PreProc,
                to_strings(&["#", "define", "lnln", "if", "0", "{", "mes", ":", "mes", "}"])
            )]
        );
    }

    #[test]
    fn statement_is_separated_by_colons() {
        let text = r#"
            pos 10, 10 : mes 1 : mes 2
        "#;
        assert_eq!(
            analyze(text),
            vec![
                (LineKind::Ground, to_strings(&["pos", "10", ",", "10"])),
                (LineKind::Ground, to_strings(&["mes", "1"])),
                (LineKind::Ground, to_strings(&["mes", "2"])),
            ],
        );
    }

    #[test]
    fn statement_is_separated_by_braces() {
        let text = r#"
            if 0 { mes 1 } mes 2
        "#;
        assert_eq!(
            analyze(text),
            vec![
                (LineKind::Ground, to_strings(&["if", "0"])),
                (LineKind::Ground, to_strings(&["mes", "1"])),
                (LineKind::Ground, to_strings(&["mes", "2"])),
            ]
        );
    }

    #[test]
    fn escaped_eol() {
        let text = r#"
            #deffunc foo int name, \
                local a

                return
        "#;
        assert_eq!(
            analyze(text),
            vec![
                (
                    LineKind::PreProc,
                    to_strings(&["#", "deffunc", "foo", "int", "name", ",", "local", "a"])
                ),
                (LineKind::Ground, to_strings(&["return"]))
            ]
        );
    }
}
