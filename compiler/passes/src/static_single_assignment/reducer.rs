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

use crate::RenameTable;

use leo_ast::{
    AssignOperation, AssignStatement, Assignee, BinaryExpression, BinaryOperation, Block, Expression,
    ExpressionReducer, Identifier, Node, ProgramReducer, Statement, StatementReducer, TypeReducer,
};
use leo_errors::Result;
use leo_span::{Span, Symbol};

pub(crate) struct StaticSingleAssignmentReducer {
    /// The `RenameTable` for the current basic block in the AST
    pub(crate) rename_table: RenameTable,
    /// A strictly increasing counter, used to ensure that new variable names are unique.
    pub(crate) counter: usize,
    /// A flag to determine whether or not the traversal is on the left-hand side of a definition or an assignment.
    pub(crate) is_lhs: bool,
    /// Phi functions produced by static single assignment.
    pub(crate) phi_functions: Vec<Statement>,
}

impl StaticSingleAssignmentReducer {
    /// Returns the value of `self.counter`. Increments the counter by 1, ensuring that all invocations of this function return a unique value.
    pub fn get_unique_id(&mut self) -> usize {
        self.counter += 1;
        self.counter - 1
    }

    /// Clears the `self.phi_functions`, returning the ones that were previously produced.
    pub fn clear_phi_functions(&mut self) -> Vec<Statement> {
        core::mem::take(&mut self.phi_functions)
    }

    /// Pushes a new scope for a child basic block.
    pub fn push(&mut self) {
        let parent_table = core::mem::take(&mut self.rename_table);
        self.rename_table = RenameTable {
            parent: Some(Box::from(parent_table)),
            mapping: Default::default(),
        };
    }

    /// If the RenameTable has a parent, then `self.rename_table` is set to the parent, otherwise it is set to a default `RenameTable`.
    pub fn pop(&mut self) -> RenameTable {
        let parent = self.rename_table.parent.clone().unwrap();
        let child_table = core::mem::replace(&mut self.rename_table, *parent);

        child_table
    }
}

impl TypeReducer for StaticSingleAssignmentReducer {}

impl ExpressionReducer for StaticSingleAssignmentReducer {
    /// Produces a new `Identifier` with a unique name.
    /// If this function is invoked on the left-hand side of a definition or assignment, a new unique name is introduced.
    /// Otherwise, we look up the previous name in the `RenameTable`.
    fn reduce_identifier(&mut self, identifier: &Identifier) -> Result<Identifier> {
        match self.is_lhs {
            true => {
                let new_name = Symbol::intern(&format!("{}${}", identifier.name, self.get_unique_id()));
                self.rename_table.update(identifier.name, new_name.clone());
                Ok(Identifier {
                    name: new_name,
                    span: identifier.span,
                })
            }
            false => {
                match self.rename_table.lookup(&identifier.name) {
                    // TODO: Better error.
                    None => panic!(
                        "Error: A unique name for the variable {} is not defined.",
                        identifier.name
                    ),
                    Some(name) => Ok(Identifier {
                        name: name.clone(),
                        span: identifier.span,
                    }),
                }
            }
        }
    }
}

impl StatementReducer for StaticSingleAssignmentReducer {
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
                            let symbol_string = format!("cond${}", self.get_unique_id());

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

impl ProgramReducer for StaticSingleAssignmentReducer {}
