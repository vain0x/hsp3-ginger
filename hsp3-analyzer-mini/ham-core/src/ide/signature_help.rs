//! シグネチャヘルプ (パラメータリストの説明表示)

use super::*;
use crate::{analysis::*, parse::*, source::*};
use lsp_types::{
    Documentation, ParameterInformation, ParameterLabel, SignatureHelp, SignatureInformation,
};

/// シグネチャヘルプを生成するために使うカーソル周辺の情報
pub(crate) struct SignatureHelpContext {
    pub(crate) signature_data: Rc<SignatureData>,
    pub(crate) arg_index: usize,
    pub(crate) ctype: bool,
}

/// シグネチャヘルプの生成に必要な処理を行う構文木ビジター
struct V {
    db: SignatureHelpDb,
    pos: Pos16,
    out: Option<SignatureHelpContext>,
}

impl V {
    fn find_signature(&self, symbol: &SymbolRc) -> Option<Rc<SignatureData>> {
        symbol.signature_opt()
    }

    fn try_resolve(&mut self, callee: &PToken, args: &[PArg], ctype: bool) {
        if self.out.is_some() {
            return;
        }

        let at_callee = callee.body.loc.range.contains_inclusive(self.pos);
        if at_callee {
            return;
        }

        let symbol = match self.db.resolve_symbol(callee.body_pos()) {
            Some(it) => it,
            None => return,
        };

        let signature_data = match self.find_signature(&symbol) {
            Some(it) => it,
            None => return,
        };

        let arg_index = args
            .iter()
            .filter_map(|a| a.comma_opt.as_ref())
            .take_while(|comma| comma.body.loc.range.end() <= self.pos)
            .count();

        self.out = Some(SignatureHelpContext {
            signature_data,
            ctype,
            arg_index,
        });
    }

    fn on_name_paren(&mut self, np: &PNameParen) {
        self.try_resolve(&np.name, &np.args, true);
    }

    fn on_command_stmt(&mut self, stmt: &PCommandStmt) {
        self.try_resolve(&stmt.command, &stmt.args, false);
    }
}

impl PVisitor for V {
    fn on_compound(&mut self, compound: &PCompound) {
        if self.out.is_some() || !compound.compute_range().contains_inclusive(self.pos) {
            return;
        }

        self.on_compound_default(compound);

        if let PCompound::Paren(np) = compound {
            self.on_name_paren(np);
        }
    }

    fn on_stmt(&mut self, stmt: &PStmt) {
        if self.out.is_some() || !stmt.compute_range().contains_inclusive(self.pos) {
            return;
        }

        self.on_stmt_default(stmt);

        if let PStmt::Command(stmt) = stmt {
            self.on_command_stmt(stmt);
        }
    }
}

pub(crate) fn signature_help(
    an: &AnalyzerRef<'_>,
    doc_interner: &DocInterner,
    uri: Url,
    position: Position,
) -> Option<SignatureHelp> {
    let (doc, pos) = from_document_position(doc_interner, &uri, position)?;

    if an.in_str_or_comment(doc, pos).unwrap_or(true) {
        return None;
    }

    let db = SignatureHelpDb::generate(an, doc);
    let syntax = an.get_syntax(doc)?;

    let ctx = {
        let mut v = V { db, pos, out: None };
        v.on_root(syntax.root);
        v.out?
    };

    let SignatureHelpContext {
        signature_data,
        ctype,
        arg_index,
    } = ctx;

    let command = NamePath::new(&signature_data.name).base;

    let mut params = vec![
        ParameterInformation {
            label: ParameterLabel::LabelOffsets([0; 2]),
            documentation: None,
        };
        signature_data.params.len()
    ];

    let signature_label = {
        let mut s = command.to_string();
        let mut sep = if ctype { "(" } else { " " };

        for (i, (ty_opt, name_opt, info_opt)) in signature_data.params.iter().enumerate() {
            s += sep;

            let start = s.len() as u32;
            match (ty_opt, &name_opt) {
                (Some(ty), Some(name)) => {
                    s += ty.to_str();
                    s += " ";
                    s += name;
                }
                (Some(ty), None) => s += ty.to_str(),
                (None, Some(name)) => s += name,
                _ => s += "???",
            }

            let end = s.len() as u32;
            params[i].label = ParameterLabel::LabelOffsets([start, end]);
            params[i].documentation = info_opt.clone().map(|s| Documentation::String(s));

            sep = ", ";
        }

        if ctype {
            s += ")";
        }
        s
    };

    Some(SignatureHelp {
        signatures: vec![SignatureInformation {
            label: signature_label,
            parameters: Some(params),
            documentation: None,
            active_parameter: None,
        }],
        active_parameter: Some(arg_index as u32),
        active_signature: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{analyzer::Analyzer, lsp_server::NO_VERSION};

    fn dummy_url(s: &str) -> Url {
        let workspace_dir = crate::test_utils::dummy_path().join("ws");
        Url::from_file_path(&workspace_dir.join(s)).unwrap()
    }

    #[test]
    fn test() {
        let mut an = Analyzer::new_standalone();

        an.open_doc(
            dummy_url("mod_signature_help.hsp"),
            NO_VERSION,
            r#"
#module
#deffunc f int a, str b
    return
#global
            "#
            .into(),
        );

        let main_uri = dummy_url("main.hsp");
        an.open_doc(
            main_uri.clone(),
            NO_VERSION,
            r#"
f 1
f 1, ""
            "#
            .into(),
        );
        let an = an.compute_ref();

        let opt = an.signature_help(
            main_uri.clone(),
            Position {
                line: 1,
                character: 2,
            },
        );
        let (label, active) = {
            let sig = opt.expect("signature_help");
            (
                sig.signatures[0].label.clone(),
                sig.active_parameter.expect("active_parameter"),
            )
        };
        assert_eq!((label, active), ("f int a, str b".into(), 0));

        let opt = an.signature_help(
            main_uri.clone(),
            Position {
                line: 2,
                character: 5,
            },
        );
        let (label, active) = {
            let sig = opt.expect("signature_help");
            (
                sig.signatures[0].label.clone(),
                sig.active_parameter.expect("active_parameter"),
            )
        };
        assert_eq!((label, active), ("f int a, str b".into(), 1));

        let opt = an.signature_help(
            main_uri,
            Position {
                line: 1,
                character: 0,
            },
        );
        assert!(opt.is_none());
    }

    #[test]
    fn call_test() {
        let mut an = Analyzer::new_standalone();

        an.open_doc(
            dummy_url("mod_signature_help.hsp"),
            NO_VERSION,
            r#"
#module
#defcfunc f int a, str b
    return 0
#global
            "#
            .into(),
        );

        let main_uri = dummy_url("main.hsp");
        an.open_doc(main_uri.clone(), NO_VERSION, r#"mes f(1, "")"#.into());
        let an = an.compute_ref();

        let opt = an.signature_help(
            main_uri.clone(),
            Position {
                line: 0,
                character: 6,
            },
        );
        let (label, active) = {
            let sig = opt.expect("signature_help");
            (
                sig.signatures[0].label.clone(),
                sig.active_parameter.expect("active_parameter"),
            )
        };
        assert_eq!((label, active), ("f(int a, str b)".into(), 0));

        let opt = an.signature_help(
            main_uri.clone(),
            Position {
                line: 0,
                character: 11,
            },
        );
        let (label, active) = {
            let sig = opt.expect("signature_help");
            (
                sig.signatures[0].label.clone(),
                sig.active_parameter.expect("active_parameter"),
            )
        };
        assert_eq!((label, active), ("f(int a, str b)".into(), 1));

        let opt = an.signature_help(
            main_uri,
            Position {
                line: 0,
                character: 5,
            },
        );
        assert!(opt.is_none());
    }
}
