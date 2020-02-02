use super::*;

pub(crate) struct SyntaxRoot {
    pub(crate) source: TokenSource,
    pub(crate) green: GreenNode,
    pub(crate) errors: Vec<SyntaxError>,
}
