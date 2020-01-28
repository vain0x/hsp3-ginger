use super::*;
use crate::syntax::*;
use std::fmt;

#[derive(Clone)]
pub(crate) struct KInt {
    pub token: TokenData,
}

impl fmt::Debug for KInt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.token.text())
    }
}

#[derive(Clone)]
pub(crate) struct KName {
    pub token: TokenData,
}

impl fmt::Debug for KName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.token.text())
    }
}

#[derive(Clone, Debug)]
pub(crate) enum KTerm {
    Omit,
    Int(KInt),
    Name(KName),
}

#[derive(Clone, Debug)]
pub(crate) struct KArgs {
    pub terms: Vec<KTerm>,
}
