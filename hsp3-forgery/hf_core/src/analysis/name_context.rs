use super::*;
use std::collections::HashMap;

#[derive(Default)]
pub(crate) struct NameContext {
    symbols: HashMap<AName, Symbol>,
    enclosing_deffuncs: HashMap<AName, Symbol>,
    enclosing_modules: HashMap<AName, Symbol>,
}

impl NameContext {
    pub(crate) fn enclosing_deffunc(&self, name: &AName) -> Option<Symbol> {
        self.enclosing_deffuncs.get(name).cloned()
    }

    fn enclosing_module(&self, name: &AName) -> Option<Symbol> {
        self.enclosing_modules.get(name).cloned()
    }

    pub(crate) fn symbol(&self, name: &AName) -> Option<Symbol> {
        self.symbols.get(name).cloned()
    }

    pub(crate) fn full_name(&self, name: &AName, symbols: &Symbols) -> String {
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
        name: AName,
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

    pub(crate) fn set_symbol(&mut self, name: AName, symbol: Symbol) {
        self.symbols.insert(name, symbol);
    }
}
