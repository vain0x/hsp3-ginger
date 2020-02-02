use super::*;

pub(crate) struct SyntaxRoot {
    pub(crate) green: GreenNode,
    pub(crate) errors: Vec<SyntaxError>,
}
