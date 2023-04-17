use super::*;
use crate::parse::*;

pub(crate) enum Diagnostic {
    Undefined,
    VarRequired,
}

type UseSiteMap = HashMap<(DocId, Pos), SymbolRc>;

pub(crate) struct Sema {
    pub(crate) use_site_map: UseSiteMap,
    pub(crate) diagnostics: Vec<(Diagnostic, Loc)>,
}

impl Sema {
    pub(crate) fn on_root(&mut self, root: &PRoot) {
        for stmt in &root.stmts {
            on_stmt(stmt, self)
        }
    }

    fn symbol(&self, loc: Loc) -> Option<SymbolRc> {
        self.use_site_map.get(&(loc.doc, loc.start())).cloned()
    }
}

fn on_stmt(stmt: &PStmt, ctx: &mut Sema) {
    match stmt {
        PStmt::Label(_) => {}
        PStmt::Assign(_) => {}
        PStmt::Command(stmt) => {
            let loc = stmt.command.body.loc;
            let symbol = match ctx.symbol(loc) {
                Some(it) => it,
                None => {
                    ctx.diagnostics.push((Diagnostic::Undefined, loc));
                    return;
                }
            };

            if let Some(signature_data) = symbol.signature_opt() {
                for (arg, _) in stmt
                    .args
                    .iter()
                    .zip(&signature_data.params)
                    .filter(|(_, (param, _, _))| param.map_or(false, |p| p.is_by_ref()))
                {
                    if arg_is_definitely_rval(arg, &ctx) {
                        let range = match arg.expr_opt.as_ref() {
                            Some(expr) => expr.compute_range(),
                            None => stmt.command.body.loc.range,
                        };
                        let loc = loc.with_range(range);
                        ctx.diagnostics.push((Diagnostic::VarRequired, loc));
                    }
                }
            }
        }
        PStmt::DefFunc(stmt) => {
            for stmt in &stmt.stmts {
                on_stmt(stmt, ctx);
            }
        }
        PStmt::Module(stmt) => {
            for stmt in &stmt.stmts {
                on_stmt(stmt, ctx);
            }
        }
        // PStmt::If
        _ => {}
    }
}

fn symbol_kind_is_definitely_rval(kind: HspSymbolKind) -> bool {
    match kind {
        HspSymbolKind::Label
        | HspSymbolKind::Const
        | HspSymbolKind::Enum
        | HspSymbolKind::DefFunc
        | HspSymbolKind::DefCFunc
        | HspSymbolKind::ModFunc
        | HspSymbolKind::ModCFunc
        | HspSymbolKind::ComInterface
        | HspSymbolKind::ComFunc => true,
        HspSymbolKind::Param(Some(param)) => match param {
            PParamTy::Var | PParamTy::Array | PParamTy::Modvar | PParamTy::Local => false,
            _ => true,
        },
        _ => false,
    }
}

fn arg_is_definitely_rval(arg: &PArg, ctx: &Sema) -> bool {
    let mut expr_opt = arg.expr_opt.as_ref();
    while let Some(expr) = expr_opt {
        match expr {
            PExpr::Compound(compound) => {
                let name = &compound.name().body;

                let symbol = match ctx.symbol(name.loc) {
                    Some(it) => it,
                    _ => return false,
                };

                return symbol_kind_is_definitely_rval(symbol.kind);
            }
            PExpr::Paren(expr) => expr_opt = expr.body_opt.as_deref(),
            _ => return true,
        }
    }
    false
}
