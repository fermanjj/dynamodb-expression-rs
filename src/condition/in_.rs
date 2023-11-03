use core::fmt::{self, Write};

use crate::operand::Operand;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct In {
    pub op: Operand,
    pub items: Vec<Operand>,
}

impl In {
    pub fn new<I, T>(op: Operand, items: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<Operand>,
    {
        Self {
            op,
            items: items.into_iter().map(Into::into).collect(),
        }
    }
}

impl fmt::Display for In {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.op.fmt(f)?;
        f.write_str(" IN (")?;

        let mut first = true;
        for item in &self.items {
            if first {
                first = false;
            } else {
                f.write_char(',')?;
            }

            item.fmt(f)?;
        }

        f.write_char(')')
    }
}
