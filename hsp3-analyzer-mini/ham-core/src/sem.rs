//! FIXME: func, cfunc, cmd

use crate::{
    analysis::{ADoc, ALoc, APos},
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
    kind: LineKind,
    doc: ADoc,
    row: usize,
    text: RcStr,
    leading: Vec<RcStr>,
    trailing: Vec<RcStr>,
    words: Vec<(APos, APos, RcStr)>,
    module_start: Option<usize>,
    module_end: Option<usize>,
    command_start: Option<usize>,
    command_end: Option<usize>,
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

fn char_is_nonident(c: char) -> bool {
    // _ と @ は識別子
    c.is_whitespace() || "+-*/%\\=~|&^!?.,:;<>()[]{}\"'$`".contains(c)
}

impl Line {
    fn is_command_decl(&self) -> bool {
        ["#deffunc", "#defcfunc", "#modfunc", "#modcfunc"]
            .iter()
            .any(|&s| self.text.as_str().contains(s))
    }

    fn is_macro_decl(&self) -> bool {
        ["#define", "#enum", "#const"]
            .iter()
            .any(|&s| self.text.as_str().contains(s))
    }
}

// 行をおおまかに分類する。
pub(crate) fn parse_as_lines(
    doc: ADoc,
    text: RcStr,
    lines: &mut Vec<Line>,
    line_count: &mut usize,
) {
    let mut leading = vec![];

    let mut pp = false;
    let mut in_multiline_str = false;
    let mut in_multiline_comment = false;

    let mut line_start = 0;

    while let Some(len) = text.as_str()[line_start..].find("\n") {
        let row = *line_count;
        let line_text = text.slice(line_start, line_start + len);
        line_start += len + 1;
        *line_count += 1;

        let lt = line_text.as_str();

        if pp || lt.trim_start().starts_with("#") {
            if pp {
                if let Some(last) = lines.last_mut() {
                    last.trailing.push(line_text.clone());
                }
            } else {
                lines.push(Line {
                    kind: LineKind::PreProc,
                    doc,
                    row,
                    text: line_text.clone(),
                    leading: leading.clone(),
                    trailing: vec![],
                    words: vec![],
                    module_start: None,
                    module_end: None,
                    command_start: None,
                    command_end: None,
                });
                leading.clear();
            }

            if lt.trim_end().ends_with("\\") {
                pp = true;
            } else {
                pp = false;
            }
            continue;
        }

        if lt.trim().chars().all(|c| c.is_whitespace())
            || lt.trim().starts_with("//")
            || lt.trim().starts_with(";")
        {
            leading.push(line_text);
            continue;
        }

        if in_multiline_comment || lt.contains("/*") {
            let mut x = 0;
            loop {
                if in_multiline_comment {
                    let n = match lt[x..].find("*/") {
                        None => break,
                        Some(n) => n,
                    };
                    x += n + 2;
                    in_multiline_comment = false;
                } else {
                    let n = match lt[x..].find("/*") {
                        None => break,
                        Some(n) => n,
                    };
                    x += 2 + n;
                    in_multiline_comment = true;
                }
            }
            leading.push(line_text);
            continue;
        }

        if in_multiline_str || lt.contains("{\"") {
            let mut x = 0;
            loop {
                if in_multiline_str {
                    let n = match lt[x..].find("\"}") {
                        None => break,
                        Some(n) => n,
                    };
                    x += n + 2;
                    in_multiline_str = false;
                } else {
                    let n = match lt[x..].find("{\"") {
                        None => break,
                        Some(n) => n,
                    };
                    x += 2 + n;
                    in_multiline_str = true;
                }
            }
            continue;
        }

        lines.push(Line {
            kind: LineKind::Ground,
            doc,
            row,
            text: line_text.clone(),
            leading: leading.clone(),
            trailing: vec![],
            words: vec![],
            module_start: None,
            module_end: None,
            command_start: None,
            command_end: None,
        });
        leading.clear();
    }
}

pub(crate) fn parse_as_words(lines: &mut Vec<Line>) {
    for line in lines.iter_mut() {
        let mut words = vec![];

        let mut i = 0;
        let mut y = 0;
        let mut text = line.text.clone();

        loop {
            while let Some(c) = text.as_str()[i..]
                .chars()
                .take_while(|&c| char_is_nonident(c))
                .next()
            {
                if text.as_str()[i..].starts_with(";") || text.as_str()[i..].starts_with("//") {
                    i = text.len();
                    break;
                }

                if text.as_str()[i..].starts_with("\"") {
                    i += 1;
                    while let Some(c) = text.as_str()[i..].chars().take_while(|&c| c != '"').next()
                    {
                        if c == '\\' {
                            i += text.as_str()[i..]
                                .chars()
                                .take(2)
                                .map(|c| c.len_utf8())
                                .sum::<usize>();
                            continue;
                        }
                        i += c.len_utf8();
                    }
                    continue;
                }

                i += c.len_utf8();
            }

            let start = i;

            while let Some(c) = text.as_str()[i..]
                .chars()
                .take_while(|&c| !char_is_nonident(c))
                .next()
            {
                i += c.len_utf8();
            }

            let end = i;

            if start == end {
                if y >= line.trailing.len() {
                    break;
                }

                text = line.trailing[y].clone();
                i = 0;
                y += 1;
                continue;
            }

            if text.as_str()[start..end]
                .chars()
                .next()
                .map_or(true, |c| c.is_ascii_digit())
            {
                continue;
            }

            let row = line.row + y;
            words.push((
                APos { row, column: start },
                APos { row, column: end },
                text.slice(start, end),
            ));
        }

        line.words = words.clone();
    }
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
            if line.text.as_str().contains("#module") {
                module_start = Some(line.row);
            }
            if line.text.as_str().contains("#global") {
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
        if line.kind != LineKind::PreProc {
            continue;
        }

        if !line.is_command_decl() {
            continue;
        }

        let mut first = true;
        let mut global = false;
        let mut local = false;
        let ctype = line.text.as_str().contains("cfunc");

        for &(start, end, ref word) in &line.words {
            let is_keyword = [
                "#deffunc",
                "#defcfunc",
                "#modfunc",
                "#modcfunc",
                "global",
                "local",
                "int",
                "str",
                "double",
                "label",
                "var",
                "array",
                "modvar",
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
        if line.kind != LineKind::PreProc {
            continue;
        }

        if !line.is_macro_decl() {
            continue;
        }

        let mut global = false;
        let mut ctype = false;

        for &(start, end, ref word) in &line.words {
            let is_keyword = [
                "#define", "#enum", "#const", "global", "local", "ctype", "double", "int",
            ]
            .contains(&word.as_str());

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
        if line.kind != LineKind::Ground {
            continue;
        }

        let text = line.text.as_str();

        if !text.trim_start().starts_with("*") {
            continue;
        }

        let mut x = match text.find("*") {
            None => continue,
            Some(x) => x,
        };

        for c in text[x..].chars().take_while(|&c| char_is_nonident(c)) {
            x += c.len_utf8();
        }

        let start = APos {
            row: line.row,
            column: x,
        };

        for c in text[x..].chars().take_while(|&c| !char_is_nonident(c)) {
            x += c.len_utf8();
        }

        let end = APos {
            row: line.row,
            column: x,
        };

        let name = line.text.slice(start.column, end.column);
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
        if line.kind != LineKind::Ground {
            continue;
        }

        let text = line.text.as_str();
        if !KEYWORDS
            .iter()
            .any(|keyword| text.contains(keyword) || text.contains("="))
        {
            continue;
        }

        let candidate = if line.words.len() >= 2 && KEYWORDS.contains(&line.words[0].2.as_str()) {
            Some(line.words[1].clone())
        } else if line.words.len() >= 1 && text[line.words[0].1.column..].trim().starts_with("=") {
            Some(line.words[0].clone())
        } else {
            None
        };

        let (start, end, word) = match candidate {
            None => continue,
            Some(x) => x,
        };

        let loc = ALoc::new3(doc, start, end);

        if symbol_map
            .get(word.as_str())
            .unwrap_or(&vec![])
            .iter()
            .any(|symbol| symbol_is_in_scope(symbol, doc, line.row))
        {
            continue;
        }

        let name = word.clone();
        let kind = SymbolKind::Static;
        let symbol_id = {
            *last_symbol_id += 1;
            *last_symbol_id
        };
        let symbol = Rc::new(Symbol {
            symbol_id,
            name,
            kind,
            details: calculate_details(&line.leading),
            scope: calculate_scope(doc, kind, line.module_start, line.module_end),
        });
        symbols.insert(symbol_id, symbol.clone());
        symbol_defs.insert(symbol_id, vec![loc]);
        symbol_map
            .entry(word.clone())
            .or_insert(vec![])
            .push(symbol);
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

    parse_as_lines(doc, text.clone(), &mut lines, &mut line_count);
    parse_as_words(&mut lines);
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
