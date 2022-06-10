// Copyright (C) 2019-2022 Aleo Systems Inc.
// This file is part of the Leo library.

// The Leo library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The Leo library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the Leo library. If not, see <https://www.gnu.org/licenses/>.

use crate::NameTable;

use leo_ast::{
    AssignOperation, AssignStatement, Assignee, BinaryExpression, BinaryOperation, Block, Expression, Identifier, Node,
    ReconstructingReducer, Statement,
};
use leo_errors::Result;
use leo_span::{Span, Symbol};

struct ReduceToSSAForm {
    name_table: NameTable,
    /// A strictly increasing counter, used to ensure that new variable names are unique.
    // Developer Note:
    //   - Using a single counter for variable renaming will produce a program that may be visually difficult to read.
    //     While this has no impact on code generation, if preferred, one can maintain a counter for each variable in the source code, at the expense of code complexity.
    counter: usize,
}

impl ReconstructingReducer for ReduceToSSAForm {
    fn in_circuit(&self) -> bool {
        // Always default to false since `circuit` is not supported in Leo programs.
        false
    }

    fn swap_in_circuit(&mut self) {
        // Do nothing since `circuit` is not supported in Leo programs.
    }

    /// Reduce all `AssignStatement`s to simple `AssignStatement`s.
    /// For example,
    ///   `x += y * 3` becomes `x = x + (y * 3)`
    ///   `x &= y | 1` becomes `x = x & (y | 1)`
    ///   `x = y + 3` remains `x = y + 3`
    // TODO: Verify that these are expected semantics.
    fn reduce_assign(
        &mut self,
        assign: &AssignStatement,
        assignee: Assignee,
        value: Expression,
    ) -> Result<AssignStatement> {
        // Helper function to construct a binary expression using `assignee` and `value` as operands.
        let reduce_to_binary_operation = |binary_operation: BinaryOperation, value: Expression| -> Expression {
            let expression_span = value.span();
            Expression::Binary(BinaryExpression {
                left: Box::new(Expression::Identifier(assignee.identifier)),
                right: Box::new(value),
                op: binary_operation,
                span: expression_span,
            })
        };

        let value = match assign.operation {
            AssignOperation::Assign => value,
            AssignOperation::Add => reduce_to_binary_operation(BinaryOperation::Add, value),
            AssignOperation::Sub => reduce_to_binary_operation(BinaryOperation::Sub, value),
            AssignOperation::Mul => reduce_to_binary_operation(BinaryOperation::Mul, value),
            AssignOperation::Div => reduce_to_binary_operation(BinaryOperation::Div, value),
            AssignOperation::Pow => reduce_to_binary_operation(BinaryOperation::Pow, value),
            AssignOperation::Or => reduce_to_binary_operation(BinaryOperation::Or, value),
            AssignOperation::And => reduce_to_binary_operation(BinaryOperation::And, value),
        };

        Ok(AssignStatement {
            operation: AssignOperation::Assign,
            assignee,
            value,
            span: Default::default(),
        })
    }

    /// This function:
    ///   - Converts all `DefinitionStatement`s to `AssignStatement`s.
    ///   - Introduces a new `AssignStatement` for non-trivial expressions in the condition of `ConditionalStatement`s.
    ///     For example,
    ///       - `if x > 0 { x = x + 1 }` becomes `let cond$0 = x > 0; if cond$0 { x = x + 1; }`
    ///       - `if true { x = x + 1 }` remains the same.
    ///       - `if b { x = x + 1 }` remains the same.
    fn reduce_block(&mut self, block: &Block, statements: Vec<Statement>) -> Result<Block> {
        let mut new_statements = Vec::with_capacity(statements.len());
        statements.into_iter().for_each(|statement| {
            match statement {
                // Rewrite a `DefinitionStatement` as an `AssignStatement`.
                Statement::Definition(definition_statement) => {
                    let assignee = Assignee {
                        identifier: definition_statement.variable_name.identifier,
                        accesses: vec![],
                        span: Default::default(),
                    };
                    let statement = Statement::Assign(Box::from(AssignStatement {
                        operation: AssignOperation::Assign,
                        assignee,
                        value: definition_statement.value,
                        span: Default::default(),
                    }));
                    new_statements.push(statement);
                }
                // TODO: Clean up logic.
                // Extract the condition of a `ConditionalStatement` and introduce a new `AssignStatement` for it, if necessary.
                Statement::Conditional(ref conditional_statement) => {
                    match conditional_statement.condition {
                        // TODO: Do we have a better way of handling unreachable errors?
                        Expression::Call(..) => {
                            unreachable!("Call expressions should not exist in the AST at this stage of compilation.")
                        }
                        Expression::Err(_) => {
                            unreachable!("Err expressions should not exist in the AST at this stage of compilation.")
                        }
                        Expression::Identifier(..) | Expression::Value(..) => new_statements.push(statement),
                        Expression::Binary(..) | Expression::Unary(..) | Expression::Ternary(..) => {
                            // Create a fresh variable name for the condition.
                            let symbol_string = format!("cond${}", self.counter);
                            self.counter += 1;

                            // Initialize a new `AssignStatement` for the condition.
                            let assignee = Assignee {
                                identifier: Identifier::new(Symbol::intern(&symbol_string)),
                                accesses: vec![],
                                span: Span::default(),
                            };
                            let assign_statement = Statement::Assign(Box::new(AssignStatement {
                                operation: AssignOperation::Assign,
                                assignee,
                                value: conditional_statement.condition.clone(),
                                span: Span::default(),
                            }));
                            new_statements.push(assign_statement);
                            new_statements.push(statement);
                        }
                    };
                }
                _ => new_statements.push(statement),
            }
        });

        Ok(Block {
            statements: new_statements,
            span: block.span,
        })
    }
}
