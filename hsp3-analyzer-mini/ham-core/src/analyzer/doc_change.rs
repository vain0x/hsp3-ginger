use super::*;

pub(crate) enum DocChange {
    Opened {
        doc: DocId,
        lang: Lang,
        origin: DocChangeOrigin,
    },
    Changed {
        doc: DocId,
        lang: Lang,
        origin: DocChangeOrigin,
    },
    Closed {
        doc: DocId,
    },
}

pub(crate) enum DocChangeOrigin {
    Editor(RcStr),
    Path(PathBuf),
}
