use super::*;
use super::{a_scope::*, preproc::*};
use crate::{
    parse::{PRoot, PToken},
    source::Loc,
    utils::{rc_slice::RcSlice, rc_str::RcStr},
};
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, PartialOrd)]
enum Phase {
    Init,
    Syntax,
    Preproc,
    Symbols,
}

impl Default for Phase {
    fn default() -> Self {
        Phase::Init
    }
}

#[derive(Default)]
pub(crate) struct DocAnalysis {
    phase: Phase,

    // 構文:
    pub(crate) tokens: RcSlice<PToken>,
    pub(crate) tree_opt: Option<PRoot>,

    // プリプロセス:
    pub(crate) includes: Vec<(RcStr, Loc)>,
    pub(crate) modules: HashMap<AModule, AModuleData>,
    pub(crate) deffuncs: HashMap<ADefFunc, ADefFuncData>,
    pub(crate) preproc_symbols_len: usize,

    /// ドキュメント内のシンボル
    pub(crate) symbols: Vec<ASymbolData>,
}

impl DocAnalysis {
    pub(crate) fn invalidate(&mut self) {
        self.phase = Phase::Init;
        self.tokens = [].into();
        self.tree_opt = None;
        self.includes.clear();
        self.modules.clear();
        self.deffuncs.clear();
        self.symbols.clear();
    }

    pub(crate) fn set_syntax(&mut self, tokens: RcSlice<PToken>, tree: PRoot) {
        debug_assert_eq!(self.phase, Phase::Init);
        self.phase = Phase::Syntax;
        self.tokens = tokens;
        self.tree_opt = Some(tree);
    }

    pub(crate) fn set_preproc(&mut self, preproc: PreprocAnalysisResult) {
        debug_assert_eq!(self.phase, Phase::Syntax);
        self.phase = Phase::Preproc;
        self.symbols = preproc.symbols;
        self.includes = preproc.includes;
        self.modules = preproc.modules;
        self.deffuncs = preproc.deffuncs;
        self.preproc_symbols_len = self.symbols.len();
    }

    pub(crate) fn rollback_to_preproc(&mut self) {
        if self.phase == Phase::Symbols {
            self.phase = Phase::Preproc;
            self.symbols.drain(self.preproc_symbols_len..);
        }
    }

    pub(crate) fn symbols_updated(&mut self) {
        debug_assert_eq!(self.phase, Phase::Preproc);
        self.phase = Phase::Symbols;
    }

    pub(crate) fn after_preproc(&self) -> bool {
        self.phase >= Phase::Preproc
    }

    pub(crate) fn after_symbols(&self) -> bool {
        self.phase == Phase::Symbols
    }
}
