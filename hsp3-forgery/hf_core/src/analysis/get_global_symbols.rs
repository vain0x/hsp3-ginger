use super::*;
use std::rc::Rc;

#[derive(Default)]
struct GlobalSymbolCollection {
    current_module_opt: Option<Rc<SyntaxNode>>,
    current_deffunc_opt: Option<Rc<SyntaxNode>>,
    symbols: Vec<GlobalSymbol>,
}

impl GlobalSymbolCollection {
    fn new() -> Self {
        Default::default()
    }
}

fn close_module(node_opt: Option<&SyntaxNode>, gsc: &mut GlobalSymbolCollection) {
    let module_stmt = match gsc.current_module_opt.take() {
        None => return,
        Some(x) => x,
    };

    gsc.symbols.push(GlobalSymbol::Module {
        module_stmt,
        global_stmt_opt: node_opt.cloned().map(Rc::new),
    });
}

fn close_deffunc(node_opt: Option<&SyntaxNode>, gsc: &mut GlobalSymbolCollection) {
    let deffunc_stmt = match gsc.current_deffunc_opt.take() {
        None => return,
        Some(x) => x,
    };

    gsc.symbols.push(GlobalSymbol::Deffunc {
        deffunc_stmt,
        closer_stmt_opt: node_opt.cloned().map(Rc::new),
    });
}

fn go(node: SyntaxNode, gsc: &mut GlobalSymbolCollection) {
    for child in node.child_nodes() {
        match child.kind() {
            NodeKind::LabelStmt => {
                gsc.symbols.push(GlobalSymbol::Label {
                    label_stmt: Rc::new(child.clone()),
                    module_stmt_opt: gsc.current_module_opt.clone(),
                });
            }
            NodeKind::ModulePp => {
                // FIXME: #deffunc の途中に #module があるケースはしばらく対応しない
                close_deffunc(Some(&child), gsc);

                // モジュールは入れ子にならないので、現在の #module は閉じる。
                close_module(Some(&child), gsc);

                gsc.current_module_opt = Some(Rc::new(child.clone()));
            }
            NodeKind::GlobalPp => {
                close_deffunc(Some(&child), gsc);
                close_module(Some(&child), gsc);
            }
            NodeKind::DeffuncPp => {
                close_deffunc(Some(&child), gsc);
                gsc.current_deffunc_opt = Some(Rc::new(child.clone()));
            }
            _ => {}
        }

        go(child, gsc);
    }
}

pub(crate) fn get_global_symbols(syntax_root: &SyntaxRoot) -> GlobalSymbols {
    let mut gsc = GlobalSymbolCollection::new();

    go(syntax_root.node(), &mut gsc);
    close_deffunc(None, &mut gsc);
    close_module(None, &mut gsc);

    GlobalSymbols::from(gsc.symbols)
}
