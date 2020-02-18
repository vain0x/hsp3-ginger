use super::*;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Default)]
struct GlobalSymbolCollection {
    current_module_opt: Option<Symbol>,
    current_deffunc_opt: Option<Symbol>,
    name_context: NameContext,
    symbols: Symbols,
}

impl GlobalSymbolCollection {
    fn new() -> Self {
        Default::default()
    }
}

fn close_module(node_opt: Option<&SyntaxNode>, gsc: &mut GlobalSymbolCollection) {
    let module_symbol = match gsc.current_module_opt.take() {
        None => return,
        Some(x) => x,
    };

    gsc.symbols.define_module(&module_symbol, node_opt.cloned());
}

fn close_deffunc(node_opt: Option<&SyntaxNode>, gsc: &mut GlobalSymbolCollection) {
    let deffunc_symbol = match gsc.current_deffunc_opt.take() {
        None => return,
        Some(x) => x,
    };

    gsc.symbols
        .define_deffunc(&deffunc_symbol, node_opt.cloned());
}

fn go(node: SyntaxNode, gsc: &mut GlobalSymbolCollection) {
    for child in node.child_nodes() {
        match child.kind() {
            NodeKind::Ident => {
                let name = AName::cast(&child).unwrap();
                gsc.name_context.set_enclosures(
                    name,
                    gsc.current_deffunc_opt.clone(),
                    gsc.current_module_opt.clone(),
                );
            }
            NodeKind::LabelStmt => {
                // gsc.symbols.push(GlobalSymbol::Label {
                //     label_stmt: Rc::new(child.clone()),
                //     module_stmt_opt: gsc.current_module_opt.clone(),
                // });
            }
            NodeKind::DeffuncPp => {
                close_deffunc(Some(&child), gsc);

                let symbol = gsc.symbols.fresh_deffunc(ADeffuncPp::cast(&child).unwrap());
                gsc.current_deffunc_opt = Some(symbol);
            }
            NodeKind::ModulePp => {
                // FIXME: #deffunc の途中に #module があるケースはしばらく対応しない
                close_deffunc(Some(&child), gsc);

                // モジュールは入れ子にならないので、現在の #module は閉じる。
                close_module(Some(&child), gsc);

                let symbol = gsc.symbols.fresh_module(AModulePp::cast(&child).unwrap());
                gsc.current_module_opt = Some(symbol);
            }
            NodeKind::GlobalPp => {
                close_deffunc(Some(&child), gsc);
                close_module(Some(&child), gsc);
            }
            _ => {}
        }

        go(child, gsc);
    }
}

pub(crate) fn get_global_symbols(syntax_root: &SyntaxRoot) -> (NameContext, Symbols) {
    let mut gsc = GlobalSymbolCollection::new();

    go(syntax_root.node(), &mut gsc);
    close_deffunc(None, &mut gsc);
    close_module(None, &mut gsc);

    (gsc.name_context, gsc.symbols)
}
