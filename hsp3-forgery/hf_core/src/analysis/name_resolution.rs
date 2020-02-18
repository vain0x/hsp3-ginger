use super::*;
use std::collections::HashMap;

type Env = HashMap<String, Symbol>;

fn create_global_env(symbols: &Symbols, env: &mut Env) {
    for symbol in symbols.iter() {
        if symbols.kind(&symbol) == SymbolKind::Param {
            continue;
        }

        if let Some(name) = symbols.unqualified_name(&symbol) {
            env.insert(name.to_string(), symbol.clone());
        }
    }
}

fn try_resolve_name(
    name: &AName,
    symbols: &Symbols,
    env: &mut Env,
    name_context: &mut NameContext,
) -> Option<Symbol> {
    // FIXME: スコープを考慮する
    let unqualified_name = name.unqualified_name();

    // パラメータか？
    if let Some(param) = name_context
        .enclosing_deffunc(&name)
        .into_iter()
        .flat_map(|deffunc| symbols.params(&deffunc))
        .filter_map(|param| {
            let param_name = symbols.unqualified_name(&param)?;
            if unqualified_name == param_name {
                Some(param)
            } else {
                None
            }
        })
        .next()
    {
        return Some(param);
    }

    env.get(&unqualified_name).cloned()
}

fn resolve_node(
    node: &SyntaxNode,
    symbols: &Symbols,
    env: &mut Env,
    name_context: &mut NameContext,
) {
    for child in node.child_nodes() {
        if let Some(name) = AName::cast(&child) {
            if let Some(symbol) = try_resolve_name(&name, symbols, env, name_context) {
                name_context.set_symbol(name, symbol);
            }
        }

        resolve_node(&child, symbols, env, name_context);
    }
}

pub(crate) fn resolve(syntax_root: &SyntaxRoot, symbols: &Symbols, name_context: &mut NameContext) {
    let mut env = Env::new();
    create_global_env(symbols, &mut env);
    resolve_node(&syntax_root.node(), symbols, &mut env, name_context);
}
