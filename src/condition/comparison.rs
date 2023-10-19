use core::fmt;

use crate::operand::Operand;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Comparison {
    pub(crate) left: Operand,
    pub(crate) cmp: Comparator,
    pub(crate) right: Operand,
}

impl fmt::Display for Comparison {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { left, cmp, right } = self;

        write!(f, "{left} {cmp} {right}")
    }
}

/**
[DynamoDB comparison operators](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/Expressions.OperatorsAndFunctions.html#Expressions.OperatorsAndFunctions.Comparators)

```no-compile
comparator ::=
    =
    | <>
    | <
    | <=
    | >
    | >=
*/
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Comparator {
    /// Equal (`=`)
    Eq,
    /// Not equal (`<>`)
    Ne,
    /// Less than (`<`)
    Lt,
    /// Less than or equal (`<=`)
    Le,
    /// Greater than (`>`)
    Gt,
    /// Greater than or equal (`>=`)
    Ge,
}

impl Comparator {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Eq => "=",
            Self::Ne => "<>",
            Self::Lt => "<",
            Self::Le => "<=",
            Self::Gt => ">",
            Self::Ge => ">=",
        }
    }
}

impl fmt::Display for Comparator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

pub fn equal<L, R>(left: L, right: R) -> Comparison
where
    L: Into<Operand>,
    R: Into<Operand>,
{
    Comparison {
        left: left.into(),
        cmp: Comparator::Eq,
        right: right.into(),
    }
}

pub fn not_equal<L, R>(left: L, right: R) -> Comparison
where
    L: Into<Operand>,
    R: Into<Operand>,
{
    Comparison {
        left: left.into(),
        cmp: Comparator::Ne,
        right: right.into(),
    }
}

pub fn less_than<L, R>(left: L, right: R) -> Comparison
where
    L: Into<Operand>,
    R: Into<Operand>,
{
    Comparison {
        left: left.into(),
        cmp: Comparator::Lt,
        right: right.into(),
    }
}

pub fn less_than_or_equal<L, R>(left: L, right: R) -> Comparison
where
    L: Into<Operand>,
    R: Into<Operand>,
{
    Comparison {
        left: left.into(),
        cmp: Comparator::Le,
        right: right.into(),
    }
}

pub fn greater_than<L, R>(left: L, right: R) -> Comparison
where
    L: Into<Operand>,
    R: Into<Operand>,
{
    Comparison {
        left: left.into(),
        cmp: Comparator::Gt,
        right: right.into(),
    }
}

pub fn greater_than_or_equal<L, R>(left: L, right: R) -> Comparison
where
    L: Into<Operand>,
    R: Into<Operand>,
{
    Comparison {
        left: left.into(),
        cmp: Comparator::Ge,
        right: right.into(),
    }
}

#[cfg(test)]
mod test {
    use pretty_assertions::assert_str_eq;

    use super::Comparator::*;

    #[test]
    fn display() {
        assert_str_eq!("=", Eq.to_string());
        assert_str_eq!("<>", Ne.to_string());
        assert_str_eq!("<", Lt.to_string());
        assert_str_eq!("<=", Le.to_string());
        assert_str_eq!(">", Gt.to_string());
        assert_str_eq!(">=", Ge.to_string());
    }
}
