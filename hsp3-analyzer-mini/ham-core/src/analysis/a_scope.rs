use crate::{
    parse::PToken,
    source::{DocId, Loc},
    token::TokenKind,
    utils::{id::Id, rc_str::RcStr},
};

pub(crate) type ADefFunc = Id<ADefFuncData>;

#[derive(Debug)]
pub(crate) struct ADefFuncData {
    pub(crate) content_loc: Loc,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct AModule {
    pub(crate) doc: DocId,
    pub(crate) index: usize,
    pub(crate) name_opt: Option<RcStr>,
}

impl AModule {
    pub(crate) fn new(doc: DocId, index: &mut usize, name_opt: &Option<PToken>) -> AModule {
        // FIXME: 識別子として有効な文字列なら名前として使える。
        let name_opt = match name_opt {
            Some(token) if token.kind() == TokenKind::Ident => Some(token.body.text.clone()),
            _ => None,
        };

        let module = AModule {
            doc,
            index: *index,
            name_opt,
        };
        *index += 1;

        module
    }
}

pub(crate) struct AModuleData {
    #[allow(unused)]
    pub(crate) keyword_loc: Loc,
    pub(crate) content_loc: Loc,
}
