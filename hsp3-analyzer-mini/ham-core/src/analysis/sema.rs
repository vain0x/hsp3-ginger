use super::*;
use crate::parse::*;

pub(crate) enum Diagnostic {
    Undefined,
    VarRequired,
}

type UseSiteMap = HashMap<(DocId, Pos), ASymbol>;

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

    fn symbol(&self, loc: Loc) -> Option<ASymbol> {
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
                for (arg, (param, _, _)) in stmt.args.iter().zip(&signature_data.params) {
                    match param {
                        Some(PParamTy::Var) | Some(PParamTy::Modvar) | Some(PParamTy::Array) => {}
                        _ => continue,
                    }

                    let mut rval = false;
                    let mut expr_opt = arg.expr_opt.as_ref();
                    while let Some(expr) = expr_opt {
                        match expr {
                            PExpr::Compound(compound) => {
                                let name = &compound.name().body;

                                let symbol = match ctx.symbol(name.loc) {
                                    Some(it) => it,
                                    _ => break,
                                };

                                rval = match symbol.kind {
                                    ASymbolKind::Label
                                    | ASymbolKind::Const
                                    | ASymbolKind::Enum
                                    | ASymbolKind::DefFunc
                                    | ASymbolKind::DefCFunc
                                    | ASymbolKind::ModFunc
                                    | ASymbolKind::ModCFunc
                                    | ASymbolKind::ComInterface
                                    | ASymbolKind::ComFunc => true,
                                    ASymbolKind::Param(Some(param)) => match param {
                                        PParamTy::Var
                                        | PParamTy::Array
                                        | PParamTy::Modvar
                                        | PParamTy::Local => false,
                                        _ => true,
                                    },
                                    _ => false,
                                };
                                break;
                            }
                            PExpr::Paren(expr) => expr_opt = expr.body_opt.as_deref(),
                            _ => {
                                rval = true;
                                break;
                            }
                        }
                    }
                    if rval {
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
        _ => {}
    }
}
