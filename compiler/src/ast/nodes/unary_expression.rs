use super::Expression;

#[derive(Debug, Clone)]
pub struct UnaryExpression {
    pub operator: String,
    pub operand: Box<Expression>,
    pub at: usize,
    pub line: usize,
}

impl UnaryExpression {
    pub fn new(
        operator: String,
        operand: Box<Expression>,
        at: usize,
        line: usize,
    ) -> UnaryExpression {
        UnaryExpression {
            operator,
            operand,
            at,
            line,
        }
    }
}
