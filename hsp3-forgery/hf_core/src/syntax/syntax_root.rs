use super::*;

pub(crate) struct SyntaxRoot {
    pub(crate) source: SyntaxSource,
    pub(crate) green: GreenNode,
    pub(crate) errors: Vec<SyntaxError>,
}
