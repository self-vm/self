use crate::ast::Expression;

#[derive(Debug, Clone)]
pub struct IndexExpression {
    pub object: Box<Expression>,
    pub index: Box<Expression>,
    pub at: usize,
    pub line: usize,
}

impl IndexExpression {
    pub fn new(
        object: Box<Expression>,
        index: Box<Expression>,
        at: usize,
        line: usize,
    ) -> IndexExpression {
        IndexExpression {
            object,
            index,
            at,
            line,
        }
    }
}
