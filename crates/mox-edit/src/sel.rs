use std::collections::BTreeSet;

/// 편집 대상 행. All이면 전체.
#[derive(Debug, Clone, Copy)]
pub enum Sel<'a> {
    All,
    Rows(&'a BTreeSet<usize>),
}

impl Sel<'_> {
    pub fn contains(&self, i: usize) -> bool {
        match self {
            Sel::All => true,
            Sel::Rows(rows) => rows.contains(&i),
        }
    }
}
