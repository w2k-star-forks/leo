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

use leo_ast::{Block, ExpressionReducer, ProgramReducer, Statement, StatementReducer, TypeReducer};
use leo_errors::Result;

use std::marker::PhantomData;

#[derive(Default)]
pub struct FlattenConditionalStatements<'a> {
    phantom: PhantomData<&'a ()>,
}

impl<'a> TypeReducer for FlattenConditionalStatements<'a> {}

impl<'a> ExpressionReducer for FlattenConditionalStatements<'a> {}

impl<'a> StatementReducer for FlattenConditionalStatements<'a> {
    /// Transforms a `BlockStatement` into a new `BlockStatement` without `ConditionalStatements`.
    /// `ConditionalStatement`s are flattened into a sequence of statements containing the if
    /// and else bodies of the original `ConditionalStatement`.
    /// For example,
    /// `if <cond> {
    ///     <stmt1>
    ///     <stmt2>
    ///  } else {
    ///     <stmt3>
    ///  }`
    /// is transformed into,
    /// `<stmt1>
    ///  <stmt2>
    ///  <stmt3>`
    fn reduce_block(&mut self, block: &Block, statements: Vec<Statement>) -> Result<Block> {
        let mut new_statements = Vec::with_capacity(statements.len());
        statements.into_iter().for_each(|statement| {
            match statement {
                // Flatten the `ConditionalStatement` and append their bodies to the list of new statements.
                Statement::Conditional(mut conditional_statement) => {
                    new_statements.append(&mut conditional_statement.block.statements);
                    if let Some(statement) = conditional_statement.next {
                        new_statements.push(*statement)
                    }
                }
                // Append any other type of statement to the list of new statements.
                _ => new_statements.push(statement),
            }
        });

        Ok(Block {
            statements: new_statements,
            span: block.span,
        })
    }
}

impl<'a> ProgramReducer for FlattenConditionalStatements<'a> {}
