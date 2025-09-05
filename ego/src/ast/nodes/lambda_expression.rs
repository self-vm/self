use crate::ast::group::Group;

use super::block::Block;

#[derive(Debug, Clone)]
pub struct LambdaExpression {
    pub parameters: Group,
    pub body: Block,
    pub at: usize,
    pub line: usize,
}

impl LambdaExpression {
    pub fn new(parameters: Group, body: Block, at: usize, line: usize) -> LambdaExpression {
        LambdaExpression {
            parameters,
            body,
            at,
            line,
        }
    }
}
