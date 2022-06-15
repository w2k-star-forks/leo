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

use crate::{StaticSingleAssignmentReducer, SymbolTable};

use leo_ast::{AssignOperation, AssignStatement, Assignee, Block, ConditionalStatement, DefinitionStatement, Expression, ExpressionReducerDirector, Function, Identifier, ProgramReducer, ProgramReducerDirector, ReducerDirector, Statement, StatementReducer, StatementReducerDirector, TernaryExpression, TypeReducerDirector, FunctionInput};
use leo_errors::Result;

use indexmap::IndexSet;
use leo_errors::emitter::Handler;
use leo_span::Symbol;

pub(crate) struct Director<'a> {
    reducer: StaticSingleAssignmentReducer<'a>,
}

impl<'a> Director<'a> {
    // Note: This implementation of `Director` does not use `symbol_table` and `handler`.
    // It may later become necessary as we iterate on the design.
    pub(crate) fn new(symbol_table: &'a mut SymbolTable<'a>, handler: &'a Handler) -> Self {
        Self {
            reducer: StaticSingleAssignmentReducer::new(symbol_table, handler),
        }
    }
}

impl<'a> ReducerDirector for Director<'a> {
    type Reducer = StaticSingleAssignmentReducer<'a>;

    fn reducer(self) -> Self::Reducer {
        self.reducer
    }

    fn reducer_ref(&mut self) -> &mut Self::Reducer {
        &mut self.reducer
    }
}

impl<'a> TypeReducerDirector for Director<'a> {}

impl<'a> ExpressionReducerDirector for Director<'a> {}

impl<'a> StatementReducerDirector for Director<'a> {
    /// Reduces the `DefinitionStatement`, setting `is_lhs` as appropriate.
    fn reduce_definition(&mut self, definition: &DefinitionStatement) -> Result<DefinitionStatement> {
        self.reducer.is_lhs = true;
        let variable_name = self.reduce_variable_name(&definition.variable_name)?;
        self.reducer.is_lhs = false;

        let type_ = self.reduce_type(&definition.type_, &definition.span)?;

        let value = self.reduce_expression(&definition.value)?;

        self.reducer_ref()
            .reduce_definition(definition, variable_name, type_, value)
    }

    /// Reduces the `AssignStatement`, setting `is_lhs` as appropriate.
    fn reduce_assign(&mut self, assign: &AssignStatement) -> Result<AssignStatement> {
        self.reducer.is_lhs = true;
        let assignee = self.reduce_assignee(&assign.assignee)?;
        self.reducer.is_lhs = false;

        let value = self.reduce_expression(&assign.value)?;

        self.reducer_ref().reduce_assign(assign, assignee, value)
    }

    /// Reduces the `ConditionalStatement`, setting the basic blocks as appropriate.
    fn reduce_conditional(&mut self, conditional: &ConditionalStatement) -> Result<ConditionalStatement> {
        let condition = self.reduce_expression(&conditional.condition)?;

        // Instantiate a `RenameTable` for the if-block.
        self.reducer.push();
        let block = self.reduce_block(&conditional.block)?;
        let if_table = self.reducer.pop();

        // Instantiate a `RenameTable` for the else-block.
        self.reducer.push();
        let next = conditional
            .next
            .as_ref()
            .map(|condition| self.reduce_statement(condition))
            .transpose()?;

        // Note that this unwrap is safe since we just created a `RenameTable` for the else-block.
        let else_table = self.reducer.pop();

        // Instantiate phi functions for the nodes written in the `ConditionalStatement`.
        let if_write_set: IndexSet<&Symbol> = IndexSet::from_iter(if_table.get_local_names().into_iter());
        let else_write_set: IndexSet<&Symbol> = IndexSet::from_iter(else_table.get_local_names().into_iter());
        let write_set = if_write_set.union(&else_write_set);

        // TODO: Better error handling.
        for symbol in write_set.into_iter() {
            let if_name = if_table
                .lookup(symbol)
                .expect(&format!("Symbol {} should exist in the program.", symbol));
            let else_name = else_table
                .lookup(symbol)
                .expect(&format!("Symbol {} should exist in the program.", symbol));

            let ternary = Expression::Ternary(TernaryExpression {
                condition: Box::new(condition.clone()),
                if_true: Box::new(Expression::Identifier(Identifier {
                    name: if_name.clone(),
                    span: Default::default(),
                })),
                if_false: Box::new(Expression::Identifier(Identifier {
                    name: else_name.clone(),
                    span: Default::default(),
                })),
                span: Default::default(),
            });

            // Create a new name for the variable written to in the `ConditionalStatement`.
            let new_name = Symbol::intern(&format!("{}${}", symbol, self.reducer.get_unique_id()));
            self.reducer.rename_table.update(*symbol.clone(), new_name.clone());

            // Create a new `AssignStatement` for the phi function.
            let assignment = Statement::Assign(Box::from(AssignStatement {
                operation: AssignOperation::Assign,
                assignee: Assignee {
                    identifier: Identifier {
                        name: new_name,
                        span: Default::default(),
                    },
                    accesses: vec![],
                    span: Default::default(),
                },
                value: ternary,
                span: Default::default(),
            }));

            self.reducer.phi_functions.push(assignment);
        }

        // Note that this does not make any modifications to the `ConditionalStatement`.
        self.reducer_ref()
            .reduce_conditional(conditional, condition, block, next)
    }

    fn reduce_block(&mut self, block: &Block) -> Result<Block> {
        let mut statements = Vec::with_capacity(block.statements.len());
        for statement in block.statements.iter() {
            statements.push(self.reduce_statement(statement)?);
            // If the statement is a `ConditionalStatement`, then add any phi functions that were produced.
            if let Statement::Conditional(..) = statement {
                statements.append(&mut self.reducer.clear_phi_functions())
            }
        }
        self.reducer_ref().reduce_block(block, statements)
    }
}

impl<'a> ProgramReducerDirector for Director<'a> {
    /// Reduces the `Function`s in the `Program`, while allocating the appropriate `RenameTable`s.
    fn reduce_function(&mut self, function: &Function) -> Result<Function> {
        // Allocate a `RenameTable` for the function.
        self.reducer.push();

        // There is no need to reduce `function.identifier`.
        let identifier = function.identifier.clone();

        // There is no need to reduce `function.inputs`.
        // However, for each input, we must add each symbol to the rename table.
        let inputs = function.input.clone();
        for input in inputs.iter() {
            match input {
                FunctionInput::Variable(function_input_variable) => {
                    self.reducer.rename_table.update(
                        function_input_variable.identifier.name.clone(),
                        function_input_variable.identifier.name.clone(),
                    );
                }
            }
        }

        // There is no need to reduce `function.output`.
        let output = function.output.clone();

        let block = self.reduce_block(&function.block)?;

        let function = self
            .reducer_ref()
            .reduce_function(function, identifier, inputs, output, block);

        // Remove the `RenameTable` for the function.
        self.reducer.pop();

        function
    }
}
