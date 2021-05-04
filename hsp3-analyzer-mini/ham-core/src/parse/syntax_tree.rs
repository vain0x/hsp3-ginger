use std::{cell::Cell, iter};

use super::*;
use crate::{
    source::{DocId, Loc, Pos, Pos16, Range},
    token::TokenKind,
    utils::rc_str::RcStr,
};

pub(crate) struct SyntaxToken<'a> {
    pub(crate) kind: TokenKind,
    pub(crate) text: RcStr,
    pub(crate) loc: Loc,
    pub(crate) parent: SyntaxParent<'a>,
}

pub(crate) struct SNameParen<'a> {
    pub(crate) name: &'a SyntaxToken<'a>,
    pub(crate) left_paren: &'a SyntaxToken<'a>,
    pub(crate) args: Vec<SArg<'a>>,
    pub(crate) right_paren_opt: Option<&'a SyntaxToken<'a>>,
}

pub(crate) struct SCommandStmt<'a> {
    pub(crate) command: &'a SyntaxToken<'a>,
    pub(crate) jump_modifier_opt: Option<(PJumpModifier, &'a SyntaxToken<'a>)>,
    pub(crate) args: Vec<PArg>,
}

pub(crate) enum SExpr<'a> {
    Literal(&'a SyntaxToken<'a>),
    NameParen(SNameParen<'a>),
}

pub(crate) struct SArg<'a> {
    pub(crate) expr_opt: Option<SExpr<'a>>,
    pub(crate) comma_opt: Option<SyntaxToken<'a>>,
}

pub(crate) enum SStmt<'a> {
    Command(SCommandStmt<'a>),
}

pub(crate) enum SyntaxKind<'a> {
    Expr(SExpr<'a>),
    ModuleStmt(&'a PModuleStmt),
    Stmt(SStmt<'a>),
    Root(&'a PRoot),
}

pub(crate) struct SyntaxElement<'a> {
    pub(crate) kind: SyntaxKind<'a>,
    pub(crate) children: Vec<SyntaxNode<'a>>,
    pub(crate) parent: SyntaxParent<'a>,
}

impl<'a> SyntaxElement<'a> {
    pub(crate) fn new(kind: SyntaxKind<'a>) -> Self {
        Self {
            kind,
            children: vec![],
            parent: Cell::default(),
        }
    }

    pub(crate) fn child_nodes(&'a self) -> impl Iterator<Item = &'a SyntaxNode<'a>> + 'a {
        self.children.iter()
    }
}

pub(crate) type SyntaxParent<'a> = Cell<Option<&'a SyntaxParentData<'a>>>;

pub(crate) struct SyntaxParentData<'a> {
    pub(crate) element: &'a SyntaxElement<'a>,
    pub(crate) index: usize,
}

pub(crate) enum SyntaxNode<'a> {
    Token(SyntaxToken<'a>),
    Element(SyntaxElement<'a>),
}

impl<'a> SyntaxNode<'a> {
    fn deep_first_token(&'a self) -> Option<&'a SyntaxToken<'a>> {
        let mut n = self;
        loop {
            match n {
                SyntaxNode::Token(token) => return Some(token),
                SyntaxNode::Element(element) => n = element.children.first()?,
            }
        }
    }

    fn deep_last_token(&'a self) -> Option<&'a SyntaxToken<'a>> {
        let mut n = self;
        loop {
            match n {
                SyntaxNode::Token(token) => return Some(token),
                SyntaxNode::Element(element) => n = element.children.last()?,
            }
        }
    }

    pub(crate) fn range(&'a self) -> Range {
        match self
            .deep_first_token()
            .and_then(|l| self.deep_last_token().map(|r| (l, r)))
        {
            Some((l, r)) => Range::from(l.loc.range.start()..r.loc.range.end()),
            None => Range::empty(Pos::new(u32::MAX, u32::MAX, u32::MAX, u32::MAX)),
        }
    }

    pub(crate) fn child_nodes(&'a self) -> impl Iterator<Item = &'a SyntaxNode<'a>> + 'a {
        match self {
            SyntaxNode::Token(_) => &[],
            SyntaxNode::Element(element) => element.children.as_slice(),
        }
        .iter()
    }
}

impl<'a> From<SyntaxElement<'a>> for SyntaxNode<'a> {
    fn from(it: SyntaxElement<'a>) -> Self {
        SyntaxNode::Element(it)
    }
}

pub(crate) struct SyntaxTree<'a> {
    pub(crate) doc: DocId,
    pub(crate) root: SyntaxElement<'a>,
}

fn on_token<'a>(token: &'a PToken, parent: &mut SyntaxElement<'a>) {
    for token in token
        .leading
        .iter()
        .chain(iter::once(token.body.as_ref()))
        .chain(token.trailing.iter())
    {
        parent.children.push(SyntaxNode::Token(SyntaxToken {
            kind: token.kind,
            text: token.text.clone(),
            loc: token.loc,
            parent: Cell::default(),
        }));
    }
}

fn on_expr<'a>(expr: &'a PExpr, parent: &mut SyntaxElement<'a>) {
    match expr {
        PExpr::Literal(literal) => {
            let token = {
                let token = literal.body.clone();
                SyntaxToken {
                    kind: token.kind,
                    text: token.text.clone(),
                    loc: token.loc,
                    parent: Cell::default(),
                }
            };
            let e = SyntaxElement::new(SyntaxKind::Expr(SExpr::Literal()));
            parent.children.push(e.into());
        }
        PExpr::Label(_) => {}
        PExpr::Compound(_) => {}
        PExpr::Paren(_) => {}
        PExpr::Prefix(_) => {}
        PExpr::Infix(_) => {}
    }
}

fn on_stmt<'a>(stmt: &'a PStmt, parent: &mut SyntaxElement<'a>) {
    match stmt {
        PStmt::Command(stmt) => {
            let mut e = SyntaxElement {
                kind: SyntaxKind::CommandStmt(stmt),
                children: vec![],
                parent: Cell::default(),
            };
            on_token(&stmt.command, &mut e);

            if let Some((_, token)) = &stmt.jump_modifier_opt {
                on_token(token, &mut e);
            }

            // for arg in &stmt.args {
            //     arg.
            // }
            parent.children.push(SyntaxNode::Element(e));
        }
        PStmt::Module(_) => {}
        _ => todo!(),
    }
}

fn build_tree<'a>(doc: DocId, root: &'a PRoot) -> SyntaxTree<'a> {
    // let mut children = vec![];
    // root.stmts
    // root.eof

    todo!()
}

// usage

fn create_param_infos(deffunc: &Symbol, symbols: &Symbols) -> Vec<String> {
    let mut params = vec![];
    let mut s = String::new();

    for param in symbols.params(deffunc) {
        if let Some(param_ty_token) = symbols.param_node(&param).param_ty() {
            // 引数を受け取らないパラメータは無視する。
            if !ParamTy::from_str(param_ty_token.text())
                .map_or(false, |param_ty| param_ty.takes_arg())
            {
                continue;
            }

            s += param_ty_token.text();
            s += " ";
        }

        match symbols.unqualified_name(&param) {
            Some(name) => s += name,
            None => s += "???",
        }

        params.push(s.clone());
        s.clear();
    }

    params
}

fn go_node<'a>(node: &'a SyntaxNode<'a>, pos: Pos16) -> bool {
    for child in node.child_nodes() {
        if !child.range().contains_inclusive(pos) {
            continue;
        }

        if go_node(&child, pos) {
            return true;
        }

        enum ArgHolder<'a> {
            CommandStmt(&'a PCommandStmt),
            NameParen(&'a PNameParen),
        }

        let (arg_holder, command) = match child {
            SyntaxNode::Element(e) => match e.kind {
                SyntaxKind::CommandStmt(stmt) => (e, &stmt.command.body),
                SyntaxKind::NameParen(np) => (e, &np.name.body),
                _ => continue,
            },
            _ => continue,
        };

        // let name = match arg_holder
        //     .child_nodes()
        //     .filter_map(|node| match node {
        //         SyntaxNode::Element(SyntaxElement { kind: SyntaxKind::Name }) => {},
        //     } AName::cast(&))
        //     .next()
        // {
        //     None => continue,
        //     Some(x) => x,
        // };

        if command.loc.range.contains_inclusive(pos) {
            continue;
        }

        let active_param_index = arg_holder
            .child_nodes()
            .filter_map(|node| {
                if node.kind() == NodeKind::Arg {
                    Some(node)
                } else {
                    None
                }
            })
            .flat_map(|node| node.child_tokens())
            .take_while(|token| token.range().start() < pos)
            .filter(|token| token.kind() == Token::Comma)
            .count();

        let params = match name_context
            .symbol(&name)
            .map(|deffunc| create_param_infos(&deffunc, symbols))
        {
            None => continue,
            Some(x) => x,
        };

        *out = Some(SignatureHelp {
            command: command.unqualified_name(),
            params,
            active_param_index,
        });
        return true;
    }

    false
}

pub(crate) fn get(
    tree: &SyntaxTree,
    position: Position,
    name_context: &NameContext,
    symbols: &Symbols,
) -> Option<SignatureHelp> {
    let mut signature_help = None;
    go_node(
        &syntax_root.node(),
        position,
        name_context,
        symbols,
        &mut signature_help,
    );
    signature_help
}
