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
    AssignStatement, Block, ConsoleArgs, ConsoleFunction, ConsoleStatement, ExpressionReducerDirector,
    ProgramReducerDirector, ReducerDirector, ReturnStatement, Statement, StatementReducer, StatementReducerDirector,
    TypeReducerDirector,
};
use leo_errors::{AstError, Result};

#[derive(Default)]
pub(crate) struct Director {
    reducer: DeadCodeEliminator,
}

impl ReducerDirector for Director {
    type Reducer = DeadCodeEliminator;

    fn reducer(self) -> Self::Reducer {
        self.reducer
    }

    fn reducer_ref(&mut self) -> &mut Self::Reducer {
        &mut self.reducer
    }
}

impl TypeReducerDirector for Director {}

impl ExpressionReducerDirector for Director {}

impl StatementReducerDirector for Director {
    /// Reduces a `ReturnStatement`. Note that all symbols in the expression of the `ReturnStatement` are critical.
    fn reduce_return(&mut self, return_statement: &ReturnStatement) -> Result<ReturnStatement> {
        self.reducer.set_critical();
        let expression = self.reduce_expression(&return_statement.expression)?;
        self.reducer.unset_critical();

        self.reducer_ref().reduce_return(return_statement, expression)
    }

    /// Reduces an `AssignStatement`. Note that if the left-hand-side of the assignment is marked, then the right-hand-side of the assignment is critical.
    fn reduce_assign(&mut self, assign: &AssignStatement) -> Result<AssignStatement> {
        let assignee = self.reduce_assignee(&assign.assignee)?;
        if self.reducer.is_marked(&assign.assignee.identifier.name) {
            self.reducer.set_critical();
        }
        let value = self.reduce_expression(&assign.value)?;
        self.reducer.unset_critical();

        self.reducer_ref().reduce_assign(assign, assignee, value)
    }

    /// Reduces a `ConsoleStatement`. Note that all symbols in the expression of the `ConsoleStatement` are critical.
    fn reduce_console(&mut self, console_function_call: &ConsoleStatement) -> Result<ConsoleStatement> {
        self.reducer.set_critical();
        let function = match &console_function_call.function {
            ConsoleFunction::Assert(expression) => ConsoleFunction::Assert(self.reduce_expression(expression)?),
            ConsoleFunction::Error(args) | ConsoleFunction::Log(args) => {
                let mut parameters = vec![];
                for parameter in args.parameters.iter() {
                    parameters.push(self.reduce_expression(parameter)?);
                }

                let formatted = ConsoleArgs {
                    string: args.string.clone(),
                    parameters,
                    span: args.span,
                };

                match &console_function_call.function {
                    ConsoleFunction::Error(_) => ConsoleFunction::Error(formatted),
                    ConsoleFunction::Log(_) => ConsoleFunction::Log(formatted),
                    _ => return Err(AstError::impossible_console_assert_call(args.span).into()),
                }
            }
        };
        self.reducer.unset_critical();

        self.reducer_ref().reduce_console(console_function_call, function)
    }

    /// Processes the block of statements in reverse order.
    fn reduce_block(&mut self, block: &Block) -> Result<Block> {
        let mut statements = vec![];
        for statement in block.statements.iter().rev() {
            match self.reduce_statement(statement)? {
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
                    // If the left-hand side of the assignment is a variable and it is marked, then it is not dead code.
                    if self.reducer.is_marked(&stmt.assignee.identifier.name) {
                        statements.push(Statement::Assign(stmt));
                    }
                }
            }
        }

        // Reverse the statements back to the original order.
        statements.reverse();

        self.reducer_ref().reduce_block(block, statements)
    }
}

impl ProgramReducerDirector for Director {}
