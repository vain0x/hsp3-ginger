use super::*;
use std::io;

#[derive(Clone)]
pub(crate) enum Diagnostic {
    InvalidChars,
    InvalidTokens,
    MissingLabelName,
    MissingParamType,
    UnclosedParen,
}

impl Diagnostic {
    fn render(&self, w: &mut impl io::Write) -> io::Result<()> {
        match self {
            Diagnostic::InvalidChars => write!(w, "この文字を解釈できません。(注意: フォージェリはまだ全角文字を解釈できません。)"),
            Diagnostic::InvalidTokens => write!(w, "この部分は文法的に解釈できないので、無視しています。(注意: フォージェリはまだ HSP3 の一部の機能にしか対応していません。)"),
            Diagnostic::MissingLabelName => write!(w, "ラベル名がありません。"),
            Diagnostic::MissingParamType => write!(w, "パラメータタイプがありません。"),
            Diagnostic::UnclosedParen => write!(w, "カッコが閉じていません。"),
        }
    }

    pub(crate) fn to_string(&self) -> String {
        let mut w = vec![];
        self.render(&mut w).ok();
        unsafe { String::from_utf8_unchecked(w) }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Severity {
    Error,
}

#[derive(Clone)]
pub(crate) struct DiagnosticData {
    kind: Diagnostic,
    severity: Severity,
    range: Range,
}

impl DiagnosticData {
    pub(crate) fn new_error(kind: Diagnostic, range: Range) -> Self {
        DiagnosticData {
            kind,
            severity: Severity::Error,
            range,
        }
    }

    pub(crate) fn kind(&self) -> &Diagnostic {
        &self.kind
    }

    pub(crate) fn range(&self) -> Range {
        self.range
    }
}

#[derive(Default)]
pub(crate) struct Diagnostics {
    inner: Vec<DiagnosticData>,
}

impl Diagnostics {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn push_error(&mut self, kind: Diagnostic, range: Range) {
        self.inner.push(DiagnosticData::new_error(kind, range));
    }

    pub(crate) fn into_iter(self) -> impl Iterator<Item = DiagnosticData> {
        self.inner.into_iter()
    }
}
