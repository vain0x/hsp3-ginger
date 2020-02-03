use super::*;
use std::fmt;
use std::rc::Rc;

pub(crate) struct SyntaxError;

pub(crate) struct SyntaxRoot {
    pub(crate) green: GreenNode,
    pub(crate) errors: Vec<SyntaxError>,
}

impl SyntaxRoot {
    pub(crate) fn new(green: GreenNode) -> Rc<SyntaxRoot> {
        Rc::new(SyntaxRoot {
            green,
            errors: vec![],
        })
    }

    pub(crate) fn green(&self) -> &GreenNode {
        &self.green
    }

    pub(crate) fn into_node(self: Rc<Self>) -> Rc<SyntaxNode> {
        SyntaxNode::from_root(self)
    }
}

impl fmt::Debug for SyntaxRoot {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.green.fmt(f)?;
        write!(f, "\n")?;

        // for error in &self.errors {
        //     write!(f, "\nERROR at {:?}\n    {}\n", error.location, error.msg)?;
        // }

        Ok(())
    }
}
