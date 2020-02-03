use super::*;
use std::fmt;

pub(crate) struct SyntaxRoot {
    pub(crate) green: GreenNode,
    pub(crate) errors: Vec<SyntaxError>,
}

impl fmt::Debug for SyntaxRoot {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.green.fmt(f)?;
        write!(f, "\n")?;

        for error in &self.errors {
            write!(f, "\nERROR at {:?}\n    {}\n", error.location, error.msg)?;
        }

        Ok(())
    }
}
