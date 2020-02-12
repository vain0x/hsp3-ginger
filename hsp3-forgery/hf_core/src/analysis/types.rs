use super::*;
use std::rc::Rc;

#[derive(Clone)]
pub(crate) enum GlobalSymbol {
    Label {
        label_stmt: Rc<SyntaxNode>,
        module_stmt_opt: Option<Rc<SyntaxNode>>,
    },
    Module {
        module_stmt: Rc<SyntaxNode>,
        global_stmt_opt: Option<Rc<SyntaxNode>>,
    },
    Deffunc {
        deffunc_stmt: Rc<SyntaxNode>,
        closer_stmt_opt: Option<Rc<SyntaxNode>>,
    },
}

#[derive(Clone, Default)]
pub(crate) struct GlobalSymbols {
    symbols: Vec<GlobalSymbol>,
}

impl From<Vec<GlobalSymbol>> for GlobalSymbols {
    fn from(symbols: Vec<GlobalSymbol>) -> Self {
        GlobalSymbols { symbols }
    }
}

impl GlobalSymbols {
    pub(crate) fn iter(&self) -> impl Iterator<Item = &GlobalSymbol> {
        self.symbols.iter()
    }
}
