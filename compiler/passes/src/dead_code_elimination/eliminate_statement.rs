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

use crate::DeadCodeEliminator;

use leo_ast::{
    AssignStatement, Block, ConsoleArgs, ConsoleFunction, ConsoleStatement, Expression, ExpressionReconstructor,
    ReturnStatement, Statement, StatementReconstructor,
};

impl<'a> StatementReconstructor for DeadCodeEliminator<'a> {
    /// Reduces a `ReturnStatement`. Note that all symbols in the expression of the `ReturnStatement` are critical.
    fn reconstruct_return(&mut self, return_statement: ReturnStatement) -> Statement {
        self.set_critical();
        let expression = self.reconstruct_expression(return_statement.expression).0;
        self.unset_critical();

        Statement::Return(ReturnStatement {
            expression,
            span: return_statement.span,
        })
    }

    /// Reduces an `AssignStatement`. Note that if the left-hand-side of the assignment is marked, then the right-hand-side of the assignment is critical.
    fn reconstruct_assign(&mut self, assign: AssignStatement) -> Statement {
        let Expression::Identifier(id) = self.reconstruct_expression(assign.place).0;
        if self.is_marked(&id.name) {
            self.set_critical();
        }
        let value = self.reconstruct_expression(assign.value).0;
        self.unset_critical();

        Statement::Assign(Box::new(AssignStatement {
            operation: assign.operation,
            place: Expression::Identifier(id),
            value,
            span: assign.span,
        }))
    }

    /// Reduces a `ConsoleStatement`. Note that all symbols in the expression of the `ConsoleStatement` are critical.
    fn reconstruct_console(&mut self, console_statement: ConsoleStatement) -> Statement {
        self.set_critical();
        let function = match console_statement.function {
            ConsoleFunction::Assert(expression) => ConsoleFunction::Assert(self.reconstruct_expression(expression).0),
            ConsoleFunction::Error(fmt) => ConsoleFunction::Error(ConsoleArgs {
                string: fmt.string,
                parameters: fmt
                    .parameters
                    .into_iter()
                    .map(|p| self.reconstruct_expression(p).0)
                    .collect(),
                span: fmt.span,
            }),
            ConsoleFunction::Log(fmt) => ConsoleFunction::Log(ConsoleArgs {
                string: fmt.string,
                parameters: fmt
                    .parameters
                    .into_iter()
                    .map(|p| self.reconstruct_expression(p).0)
                    .collect(),
                span: fmt.span,
            }),
        };
        self.unset_critical();

        Statement::Console(ConsoleStatement {
            function,
            span: console_statement.span,
        })
    }

    /// Processes the block of statements in reverse order.
    fn reconstruct_block(&mut self, block: Block) -> Block {
        let mut statements = vec![];
        for statement in block.statements.into_iter().rev() {
            match self.reconstruct_statement(statement) {
                Statement::Definition(..) => {
                    unreachable!("`DefinitionStatement`s should not exist in the AST at this stage of compilation.")
                }
                Statement::Conditional(_) => {
                    unreachable!("`ConditionalStatement`s should not exist in the AST at this stage of compilation.")
                }
                Statement::Iteration(_) => {
                    unreachable!("`IterationStatement`s should not exist in the AST at this stage of compilation.")
                }
                Statement::Return(stmt) => {
                    statements.push(Statement::Return(stmt));
                }
                Statement::Console(stmt) => {
                    statements.push(Statement::Console(stmt));
                }
                Statement::Block(stmt) => {
                    statements.push(Statement::Block(stmt));
                }
                Statement::Assign(stmt) => {
                    match stmt.place {
                        Expression::Identifier(id) => {
                            // If the left-hand side of the assignment is a variable and it is marked, then it is not dead code.
                            if self.is_marked(&id.name) {
                                statements.push(Statement::Assign(stmt));
                            }
                        }
                        _ => {
                            unreachable!("`AssignStatement`s should only contain `Identifier`s in the left-hand side.")
                        }
                    }
                }
            }
        }

        // Reverse the statements back to the original order.
        statements.reverse();

        Block {
            statements,
            span: block.span,
        }
    }
}
