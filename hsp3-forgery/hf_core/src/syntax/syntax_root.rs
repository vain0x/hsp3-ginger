use super::*;
use std::fmt;
use std::rc::Rc;

#[derive(Clone)]
pub(crate) struct SyntaxError;

/// 具象構文木。
#[derive(Clone)]
pub(crate) struct SyntaxRoot {
    pub(crate) green: Rc<GreenNode>,
    pub(crate) errors: Vec<SyntaxError>,
}

impl SyntaxRoot {
    pub(crate) fn new(green: GreenNode) -> Rc<SyntaxRoot> {
        Rc::new(SyntaxRoot {
            green: Rc::new(green),
            errors: vec![],
        })
    }

    pub(crate) fn green(&self) -> &GreenNode {
        &self.green
    }

    pub(crate) fn range(&self) -> Range {
        let start = Position::default();
        let end = self.green().position();
        Range::new(start, end)
    }

    pub(crate) fn node(&self) -> SyntaxNode {
        SyntaxNode::from_root(Rc::new(self.clone()))
    }

    pub(crate) fn source(&self) -> &TokenSource {
        self.green()
            .children()
            .iter()
            .filter_map(|element| match element {
                GreenElement::Token(token) if token.token() == Token::Eof => Some(token.source()),
                _ => None,
            })
            .next()
            .unwrap()
    }
}

impl fmt::Debug for SyntaxRoot {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.node().fmt(f)?;
        write!(f, "\n")?;

        // for error in &self.errors {
        //     write!(f, "\nERROR at {:?}\n    {}\n", error.location, error.msg)?;
        // }

        Ok(())
    }
}
