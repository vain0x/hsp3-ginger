use super::*;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum SymbolKind {
    StaticVar,
    Label,
    Deffunc,
    Param,
    Module,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct Symbol(usize);

#[derive(Default)]
pub(crate) struct Symbols {
    symbols: Vec<Symbol>,
    kinds: HashMap<Symbol, SymbolKind>,
    declared: HashSet<Symbol>,
    unqualified_names: HashMap<Symbol, String>,
    def_sites: HashMap<Symbol, Vec<SyntaxNode>>,
    closing_sites: HashMap<Symbol, Vec<SyntaxNode>>,
}

impl Symbols {
    fn fresh_symbol(&mut self, kind: SymbolKind) -> Symbol {
        let symbol = Symbol(self.symbols.len());
        self.kinds.insert(symbol.clone(), kind);
        self.declared.insert(symbol.clone());
        self.symbols.push(symbol.clone());
        symbol
    }

    fn kind(&self, symbol: &Symbol) -> SymbolKind {
        *self.kinds.get(symbol).unwrap()
    }

    pub(crate) fn unqualified_name(&self, symbol: &Symbol) -> Option<&str> {
        self.unqualified_names.get(symbol).map(|s| s.as_str())
    }

    pub(crate) fn def_sites(&self, symbol: &Symbol) -> impl Iterator<Item = &SyntaxNode> {
        self.def_sites.get(symbol).into_iter().flatten()
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &Symbol> {
        self.symbols.iter()
    }

    fn do_define(&mut self, symbol: &Symbol) {
        self.declared.remove(symbol);
    }

    fn add_unqualified_name(&mut self, symbol: &Symbol, name: String) {
        self.unqualified_names.insert(symbol.clone(), name);
    }

    fn add_def_site(&mut self, symbol: &Symbol, def_site: SyntaxNode) {
        self.def_sites
            .entry(symbol.clone())
            .or_default()
            .push(def_site);
    }

    fn add_closing_site(&mut self, symbol: &Symbol, closing_site: SyntaxNode) {
        self.closing_sites
            .entry(symbol.clone())
            .or_default()
            .push(closing_site);
    }

    pub(crate) fn fresh_deffunc(&mut self, deffunc_stmt: ADeffuncPp) -> Symbol {
        let symbol = self.fresh_symbol(SymbolKind::Deffunc);

        if let Some(name) = deffunc_stmt.name() {
            self.add_unqualified_name(&symbol, name.unqualified_name());
            self.add_def_site(&symbol, name.syntax().clone());
        }

        symbol
    }

    pub(crate) fn define_deffunc(&mut self, symbol: &Symbol, closing_site_opt: Option<SyntaxNode>) {
        self.do_define(symbol);

        if let Some(closing_site) = closing_site_opt {
            self.add_closing_site(symbol, closing_site);
        }
    }

    pub(crate) fn fresh_module(&mut self, module_stmt: AModulePp) -> Symbol {
        let symbol = self.fresh_symbol(SymbolKind::Module);

        if let Some(module_name) = module_stmt.name() {
            self.add_unqualified_name(&symbol, module_name.to_string());
            self.add_def_site(&symbol, module_name.syntax().clone());
        }

        symbol
    }

    pub(crate) fn define_module(&mut self, symbol: &Symbol, closing_site_opt: Option<SyntaxNode>) {
        assert_eq!(self.kind(symbol), SymbolKind::Module);

        self.do_define(symbol);

        if let Some(closing_site) = closing_site_opt {
            self.add_closing_site(symbol, closing_site);
        }
    }
}

#[derive(Default)]
pub(crate) struct NameContext {
    symbols: HashMap<AIdent, Symbol>,
    enclosing_deffuncs: HashMap<AIdent, Symbol>,
    enclosing_modules: HashMap<AIdent, Symbol>,
}

impl NameContext {
    fn enclosing_deffunc(&self, name: &AIdent) -> Option<Symbol> {
        self.enclosing_deffuncs.get(name).cloned()
    }

    fn enclosing_module(&self, name: &AIdent) -> Option<Symbol> {
        self.enclosing_modules.get(name).cloned()
    }

    pub(crate) fn symbol(&self, name: &AIdent) -> Option<Symbol> {
        self.symbols.get(name).cloned()
    }

    pub(crate) fn full_name(&self, name: &AIdent, symbols: &Symbols) -> String {
        let unqualified_name = name.unqualified_name();
        let scope_name = name
            .scope_name()
            .or_else(|| {
                let module = self.enclosing_module(name)?;
                let name = symbols.unqualified_name(&module)?;
                Some(name.to_string())
            })
            .unwrap_or(String::new());
        format!("{}@{}", unqualified_name, scope_name)
    }

    pub(crate) fn set_enclosures(
        &mut self,
        name: AIdent,
        deffunc_opt: Option<Symbol>,
        module_opt: Option<Symbol>,
    ) {
        if let Some(deffunc) = deffunc_opt {
            self.enclosing_deffuncs.insert(name.clone(), deffunc);
        }

        if let Some(module) = module_opt {
            self.enclosing_modules.insert(name, module);
        }
    }

    pub(crate) fn set_symbol(&mut self, name: AIdent, symbol: Symbol) {
        self.symbols.insert(name, symbol);
    }
}
