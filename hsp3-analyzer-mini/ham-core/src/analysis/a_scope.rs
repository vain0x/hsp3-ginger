use super::*;

pub(crate) type DefFuncMap = HashMap<DefFuncKey, DefFuncData>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct DefFuncKey {
    pub(crate) doc: DocId,
    pub(crate) index: usize,
}

impl DefFuncKey {
    pub(crate) fn new(doc: DocId, index: usize) -> Self {
        Self { doc, index }
    }
}

pub(crate) struct DefFuncData {
    pub(crate) content_loc: Loc,
}

pub(crate) type ModuleMap = HashMap<ModuleKey, ModuleRc>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ModuleKey {
    pub(crate) doc: DocId,
    pub(crate) index: usize,
}

impl ModuleKey {
    pub(crate) fn new(doc: DocId, index: usize) -> ModuleKey {
        Self { doc, index }
    }
}

#[derive(Clone)]
pub(crate) struct ModuleRc(Rc<ModuleData>);

impl ModuleRc {
    pub(crate) fn new(data: ModuleData) -> Self {
        Self(Rc::new(data))
    }
}

impl Deref for ModuleRc {
    type Target = ModuleData;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

pub(crate) struct ModuleData {
    pub(crate) name_opt: Option<RcStr>,
    pub(crate) content_loc: Loc,
}

/// 文字列リテラルを識別子とみなす。
fn str_as_module_name_ident(s: &RcStr) -> Option<RcStr> {
    // "..." の形で、引用符の間に1文字以上必要。
    if s.len() <= 2 || !s.starts_with('"') || !s.ends_with('"') {
        return None;
    }

    // 数字で始まらないこと。
    if s.chars().next().unwrap().is_ascii_digit() {
        return None;
    }

    // モジュール名として許可されない文字を含まないこと。(`@` も不許可。)
    let ok = s[1..s.len() - 1]
        .chars()
        .all(|c| "_`".contains(c) || (!c.is_ascii_punctuation() && !c.is_control()));
    if !ok {
        return None;
    }

    Some(s.slice(1, s.len() - 1))
}

pub(crate) fn module_name_as_ident(token: &TokenData) -> Option<RcStr> {
    match token.kind {
        TokenKind::Ident => Some(token.text.clone()),
        TokenKind::Str => str_as_module_name_ident(&token.text),
        _ => None,
    }
}
