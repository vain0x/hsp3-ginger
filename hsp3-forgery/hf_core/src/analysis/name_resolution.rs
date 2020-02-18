use super::*;
use std::collections::HashMap;

type Env = HashMap<String, Symbol>;

fn create_global_env(symbols: &Symbols, env: &mut Env) {
    for symbol in symbols.iter() {
        if let Some(name) = symbols.unqualified_name(&symbol) {
            env.insert(name.to_string(), symbol.clone());
        }
    }
}

fn resolve_node(
    node: &SyntaxNode,
    symbols: &Symbols,
    env: &mut Env,
    name_context: &mut NameContext,
) {
    for child in node.child_nodes() {
        if let Some(name) = AIdent::cast(&child) {
            // FIXME: スコープを考慮する
            let unqualified_name = name.unqualified_name();
            if let Some(symbol) = env.get(&unqualified_name) {
                name_context.set_symbol(name, symbol.clone());
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
