use super::{p_token::PToken, PConstTy, PJumpModifier, PParamTy, PPrivacy};
use std::fmt::{self, Debug, Formatter};

fn debug_fmt_opt<T: Debug>(
    opt: Option<&T>,
    placeholder: &str,
    f: &mut Formatter<'_>,
) -> fmt::Result {
    match opt {
        Some(value) => Debug::fmt(value, f),
        None => write!(f, "?{}?", placeholder),
    }
}

#[must_use]
pub(crate) struct PLabel {
    pub(crate) star: PToken,
    pub(crate) name_opt: Option<PToken>,
}

impl Debug for PLabel {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "*")?;
        debug_fmt_opt(self.name_opt.as_ref(), "name", f)
    }
}

#[must_use]
pub(crate) struct PArg {
    pub(crate) expr_opt: Option<PExpr>,
    pub(crate) comma_opt: Option<PToken>,
}

impl Debug for PArg {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        debug_fmt_opt(self.expr_opt.as_ref(), "expr", f)?;
        debug_fmt_opt(self.comma_opt.as_ref(), ",", f)
    }
}

#[must_use]
pub(crate) struct PDotArg {
    pub(crate) dot: PToken,
    pub(crate) expr_opt: Option<PExpr>,
}

impl Debug for PDotArg {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, ".")?;
        debug_fmt_opt(self.expr_opt.as_ref(), "expr", f)
    }
}

/// `a.i.j` など。
/// (HSP2 における配列要素の参照。)
#[must_use]
pub(crate) struct PNameDot {
    pub(crate) name: PToken,
    pub(crate) args: Vec<PDotArg>,
}

impl Debug for PNameDot {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.name, f)?;
        for arg in &self.args {
            writeln!(f)?;
            Debug::fmt(arg, f)?;
        }
        Ok(())
    }
}

/// `a(i)` や `f(x, y)` など。
/// (配列要素の参照と関数の呼び出しは構文的に同じであり、ここでは区別していない。)
#[must_use]
pub(crate) struct PNameParen {
    pub(crate) name: PToken,
    pub(crate) left_paren: PToken,
    pub(crate) args: Vec<PArg>,
    pub(crate) right_paren_opt: Option<PToken>,
}

impl Debug for PNameParen {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.name, f)?;

        let mut tuple = f.debug_tuple("");
        for arg in &self.args {
            tuple.field(arg);
        }
        tuple.finish()?;

        Ok(())
    }
}

/// 複合項
/// (変数の参照、配列要素の参照、関数の呼び出し)
#[must_use]
pub(crate) enum PCompound {
    Name(PToken),
    Paren(PNameParen),
    Dots(PNameDot),
}

impl Debug for PCompound {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            PCompound::Name(it) => Debug::fmt(it, f),
            PCompound::Paren(it) => Debug::fmt(it, f),
            PCompound::Dots(it) => Debug::fmt(it, f),
        }
    }
}

/// 丸カッコで囲まれた式
#[must_use]
pub(crate) struct PGroupExpr {
    pub(crate) left_paren: PToken,
    pub(crate) body_opt: Option<Box<PExpr>>,
    pub(crate) right_paren_opt: Option<PToken>,
}

impl Debug for PGroupExpr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "group(")?;
        debug_fmt_opt(self.body_opt.as_ref(), "expr", f)?;
        write!(f, ")")
    }
}

/// 前置式 (マイナスの式)
#[must_use]
pub(crate) struct PPrefixExpr {
    pub(crate) prefix: PToken,
    pub(crate) arg_opt: Option<Box<PExpr>>,
}

impl Debug for PPrefixExpr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "prefix ")?;
        Debug::fmt(&self.prefix, f)?;
        write!(f, " ")?;
        debug_fmt_opt(self.arg_opt.as_ref(), "expr", f)
    }
}

/// 中置式 (二項演算の式)
#[must_use]
pub(crate) struct PInfixExpr {
    pub(crate) left: Box<PExpr>,
    pub(crate) infix: PToken,
    pub(crate) right_opt: Option<Box<PExpr>>,
}

impl Debug for PInfixExpr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "infix ")?;
        Debug::fmt(&self.infix, f)?;

        let mut map = f.debug_struct("");
        map.field("left", &self.left);
        map.field(
            "right",
            match &self.right_opt {
                Some(expr) => expr,
                None => &"?expr?",
            },
        );
        map.finish()
    }
}

#[must_use]
pub(crate) enum PExpr {
    Literal(PToken),
    Label(PLabel),
    Compound(PCompound),
    Group(PGroupExpr),
    Prefix(PPrefixExpr),
    Infix(PInfixExpr),
}

impl Debug for PExpr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            PExpr::Literal(it) => Debug::fmt(it, f),
            PExpr::Label(it) => Debug::fmt(it, f),
            PExpr::Compound(it) => Debug::fmt(it, f),
            PExpr::Group(it) => Debug::fmt(it, f),
            PExpr::Prefix(it) => Debug::fmt(it, f),
            PExpr::Infix(it) => Debug::fmt(it, f),
        }
    }
}

/// 代入文
#[must_use]
pub(crate) struct PAssignStmt {
    pub(crate) left: PCompound,
    pub(crate) op_opt: Option<PToken>,
    pub(crate) args: Vec<PArg>,
}

impl Debug for PAssignStmt {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "assign ")?;
        debug_fmt_opt(self.op_opt.as_ref(), "op", f)?;
        write!(f, " ")?;
        Debug::fmt(&self.left, f)?;
        write!(f, " ")?;
        f.debug_list().entries(&self.args).finish()
    }
}

/// 命令文
#[must_use]
pub(crate) struct PCommandStmt {
    pub(crate) command: PToken,
    pub(crate) jump_modifier_opt: Option<(PJumpModifier, PToken)>,
    pub(crate) args: Vec<PArg>,
}

impl Debug for PCommandStmt {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "command ")?;
        Debug::fmt(&self.command, f)?;
        write!(f, " ")?;

        if let Some((_, token)) = &self.jump_modifier_opt {
            Debug::fmt(token, f)?;
            write!(f, " ")?;
        }

        f.debug_list().entries(&self.args).finish()
    }
}

/// メソッド呼び出し文 (`ie->"navigate" url, ...` など)
#[derive(Debug)]
#[must_use]
pub(crate) struct PInvokeStmt {
    pub(crate) left: PCompound,
    pub(crate) arrow_opt: Option<PToken>,
    pub(crate) method_opt: Option<PExpr>,
    pub(crate) args: Vec<PArg>,
}

#[derive(Debug)]
#[must_use]
pub(crate) struct PConstStmt {
    pub(crate) hash: PToken,
    pub(crate) keyword: PToken,
    pub(crate) privacy_opt: Option<(PPrivacy, PToken)>,
    pub(crate) ty_opt: Option<(PConstTy, PToken)>,
    pub(crate) name_opt: Option<PToken>,
    pub(crate) init_opt: Option<PExpr>,
}

#[must_use]
pub(crate) struct PMacroParam {
    pub(crate) percent_opt: Option<PToken>,
    pub(crate) number_opt: Option<PToken>,
    pub(crate) equal_opt: Option<PToken>,
    pub(crate) init: Vec<PToken>,
    pub(crate) comma_opt: Option<PToken>,
}

impl Debug for PMacroParam {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        debug_fmt_opt(self.percent_opt.as_ref(), "%", f)?;
        debug_fmt_opt(self.number_opt.as_ref(), "number", f)?;
        debug_fmt_opt(self.equal_opt.as_ref(), "=", f)?;
        Debug::fmt(&self.init, f)?;
        debug_fmt_opt(self.comma_opt.as_ref(), ",", f)
    }
}

#[derive(Debug)]
#[must_use]
pub(crate) struct PDefineStmt {
    pub(crate) hash: PToken,
    pub(crate) keyword: PToken,
    pub(crate) privacy_opt: Option<(PPrivacy, PToken)>,
    pub(crate) ctype_opt: Option<PToken>,
    pub(crate) name_opt: Option<PToken>,
    pub(crate) left_paren_opt: Option<PToken>,
    pub(crate) params: Vec<PMacroParam>,
    pub(crate) right_paren_opt: Option<PToken>,
    pub(crate) tokens: Vec<PToken>,
}

#[derive(Debug)]
#[must_use]
pub(crate) struct PEnumStmt {
    pub(crate) hash: PToken,
    pub(crate) keyword: PToken,
    pub(crate) privacy_opt: Option<(PPrivacy, PToken)>,
    pub(crate) name_opt: Option<PToken>,
    pub(crate) equal_opt: Option<PToken>,
    pub(crate) init_opt: Option<PExpr>,
}

#[must_use]
pub(crate) struct PParam {
    pub(crate) param_ty_opt: Option<(PParamTy, PToken)>,
    pub(crate) name_opt: Option<PToken>,
    pub(crate) comma_opt: Option<PToken>,
}

impl Debug for PParam {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.param_ty_opt {
            Some((_, param_ty)) => {
                Debug::fmt(param_ty, f)?;
                write!(f, " ")?;
            }
            None => write!(f, "?param_ty? ")?,
        }

        match &self.name_opt {
            Some(name) => Debug::fmt(name, f)?,
            None => write!(f, "?name? ")?,
        }

        write!(f, "{}", if self.comma_opt.is_some() { "," } else { "?,?" })
    }
}

/// `#deffunc`, etc.
#[derive(Debug)]
#[must_use]
pub(crate) struct PDefFuncStmt {
    pub(crate) hash: PToken,
    pub(crate) keyword: PToken,
    pub(crate) privacy_opt: Option<(PPrivacy, PToken)>,
    pub(crate) name_opt: Option<PToken>,
    pub(crate) params: Vec<PParam>,
    pub(crate) onexit_opt: Option<PToken>,
    pub(crate) stmts: Vec<PStmt>,
}

#[derive(Debug)]
#[must_use]
pub(crate) struct PUseLibStmt {
    pub(crate) hash: PToken,
    pub(crate) keyword: PToken,
    pub(crate) file_path_opt: Option<PToken>,
}

/// `#func` or `#cfunc`
#[derive(Debug)]
#[must_use]
pub(crate) struct PLibFuncStmt {
    pub(crate) hash: PToken,
    pub(crate) keyword: PToken,
    pub(crate) privacy_opt: Option<(PPrivacy, PToken)>,
    pub(crate) name_opt: Option<PToken>,
    pub(crate) onexit_opt: Option<PToken>,
    /// DLL からエクスポートされる関数の名前 (識別子または文字列)
    pub(crate) func_name_opt: Option<PToken>,
    /// 関数の引数の型を表す整数リテラル
    /// (HSP2.x 時代の機能)
    pub(crate) type_id_opt: Option<PToken>,
    pub(crate) params: Vec<PParam>,
}

#[derive(Debug)]
#[must_use]
pub(crate) struct PUseComStmt {
    pub(crate) hash: PToken,
    pub(crate) keyword: PToken,
    pub(crate) privacy_opt: Option<(PPrivacy, PToken)>,
    /// インターフェイス名
    pub(crate) name_opt: Option<PToken>,
    pub(crate) args: Vec<PArg>,
}

#[derive(Debug)]
#[must_use]
pub(crate) struct PComFuncStmt {
    pub(crate) hash: PToken,
    pub(crate) keyword: PToken,
    pub(crate) privacy_opt: Option<(PPrivacy, PToken)>,
    pub(crate) name_opt: Option<PToken>,
    pub(crate) index_opt: Option<PToken>,
    pub(crate) params: Vec<PParam>,
}

#[derive(Debug)]
#[must_use]
pub(crate) struct PRegCmdStmt {
    pub(crate) hash: PToken,
    pub(crate) keyword: PToken,
    pub(crate) args: Vec<PArg>,
}

/// `#cmd`
#[derive(Debug)]
#[must_use]
pub(crate) struct PCmdStmt {
    pub(crate) hash: PToken,
    pub(crate) keyword: PToken,
    pub(crate) privacy_opt: Option<(PPrivacy, PToken)>,
    pub(crate) name_opt: Option<PToken>,
    pub(crate) command_id_opt: Option<PToken>,
}

/// モジュール文。
/// `#module` から `#global` まで。
/// ファイル内に対応する `#global` がなければファイルの末尾まで。
#[derive(Debug)]
#[must_use]
pub(crate) struct PModuleStmt {
    pub(crate) hash: PToken,
    pub(crate) keyword: PToken,
    pub(crate) name_opt: Option<PToken>,
    pub(crate) fields: Vec<PParam>,
    pub(crate) stmts: Vec<PStmt>,
    pub(crate) global_opt: Option<PGlobalStmt>,
}

#[derive(Debug)]
#[must_use]
pub(crate) struct PGlobalStmt {
    pub(crate) hash: PToken,
    pub(crate) keyword: PToken,
}

/// `#include` or `#addition`
#[derive(Debug)]
#[must_use]
pub(crate) struct PIncludeStmt {
    pub(crate) hash: PToken,
    pub(crate) keyword: PToken,
    pub(crate) file_path_opt: Option<PToken>,
    pub(crate) is_optional: bool,
}

#[derive(Debug)]
#[must_use]
pub(crate) struct PUnknownPreProcStmt {
    pub(crate) hash: PToken,
    pub(crate) tokens: Vec<PToken>,
}

#[must_use]
pub(crate) enum PStmt {
    Label(PLabel),
    Assign(PAssignStmt),
    Command(PCommandStmt),
    Invoke(PInvokeStmt),
    Const(PConstStmt),
    Define(PDefineStmt),
    Enum(PEnumStmt),
    DefFunc(PDefFuncStmt),
    UseLib(PUseLibStmt),
    LibFunc(PLibFuncStmt),
    UseCom(PUseComStmt),
    ComFunc(PComFuncStmt),
    RegCmd(PRegCmdStmt),
    Cmd(PCmdStmt),
    Module(PModuleStmt),
    Global(PGlobalStmt),
    Include(PIncludeStmt),
    UnknownPreProc(PUnknownPreProcStmt),
}

impl Debug for PStmt {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            PStmt::Label(it) => Debug::fmt(it, f),
            PStmt::Assign(it) => Debug::fmt(it, f),
            PStmt::Command(it) => Debug::fmt(it, f),
            PStmt::Invoke(it) => Debug::fmt(it, f),
            PStmt::Const(it) => Debug::fmt(it, f),
            PStmt::Define(it) => Debug::fmt(it, f),
            PStmt::Enum(it) => Debug::fmt(it, f),
            PStmt::DefFunc(it) => Debug::fmt(it, f),
            PStmt::UseLib(it) => Debug::fmt(it, f),
            PStmt::LibFunc(it) => Debug::fmt(it, f),
            PStmt::UseCom(it) => Debug::fmt(it, f),
            PStmt::ComFunc(it) => Debug::fmt(it, f),
            PStmt::RegCmd(it) => Debug::fmt(it, f),
            PStmt::Cmd(it) => Debug::fmt(it, f),
            PStmt::Module(it) => Debug::fmt(it, f),
            PStmt::Global(it) => Debug::fmt(it, f),
            PStmt::Include(it) => Debug::fmt(it, f),
            PStmt::UnknownPreProc(it) => Debug::fmt(it, f),
        }
    }
}

#[derive(Debug)]
#[must_use]
pub(crate) struct PRoot {
    pub(crate) stmts: Vec<PStmt>,
    pub(crate) skipped: Vec<PToken>,
    pub(crate) eof: PToken,
}
