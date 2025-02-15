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

use crate::Flattener;
use itertools::Itertools;

use leo_ast::{
    AccessExpression, Expression, ExpressionReconstructor, Member, MemberAccess, Statement, StructExpression,
    StructVariableInitializer, TernaryExpression, TupleExpression,
};

// TODO: Clean up logic. To be done in a follow-up PR (feat/tuples)

impl ExpressionReconstructor for Flattener<'_> {
    type AdditionalOutput = Vec<Statement>;

    /// Reconstructs ternary expressions over tuples and structs, accumulating any statements that are generated.
    /// This is necessary because Aleo instructions does not support ternary expressions over composite data types.
    /// For example, the ternary expression `cond ? (a, b) : (c, d)` is flattened into the following:
    /// ```leo
    /// let var$0 = cond ? a : c;
    /// let var$1 = cond ? b : d;
    /// (var$0, var$1)
    /// ```
    /// For structs, the ternary expression `cond ? a : b`, where `a` and `b` are both structs `Foo { bar: u8, baz: u8 }`, is flattened into the following:
    /// ```leo
    /// let var$0 = cond ? a.bar : b.bar;
    /// let var$1 = cond ? a.baz : b.baz;
    /// let var$2 = Foo { bar: var$0, baz: var$1 };
    /// var$2
    /// ```
    fn reconstruct_ternary(&mut self, input: TernaryExpression) -> (Expression, Self::AdditionalOutput) {
        let mut statements = Vec::new();
        match (*input.if_true, *input.if_false) {
            // Folds ternary expressions over tuples into a tuple of ternary expression.
            // Note that this branch is only invoked when folding a conditional returns.
            (Expression::Tuple(first), Expression::Tuple(second)) => {
                let tuple = Expression::Tuple(TupleExpression {
                    elements: first
                        .elements
                        .into_iter()
                        .zip_eq(second.elements.into_iter())
                        .map(|(if_true, if_false)| {
                            // Reconstruct the true case.
                            let (if_true, stmts) = self.reconstruct_expression(if_true);
                            statements.extend(stmts);

                            // Reconstruct the false case.
                            let (if_false, stmts) = self.reconstruct_expression(if_false);
                            statements.extend(stmts);

                            // Construct a new ternary expression for the tuple element.
                            let (ternary, stmts) = self.reconstruct_ternary(TernaryExpression {
                                condition: input.condition.clone(),
                                if_true: Box::new(if_true),
                                if_false: Box::new(if_false),
                                span: input.span,
                            });

                            // Accumulate any statements generated.
                            statements.extend(stmts);

                            // Create and accumulate an intermediate assignment statement for the ternary expression corresponding to the tuple element.
                            let (identifier, statement) = self.unique_simple_assign_statement(ternary);
                            statements.push(statement);

                            // Return the identifier associated with the folded tuple element.
                            Expression::Identifier(identifier)
                        })
                        .collect(),
                    span: Default::default(),
                });
                (tuple, statements)
            }
            // If both expressions are access expressions which themselves are structs, construct ternary expression for nested struct member.
            (
                Expression::Access(AccessExpression::Member(first)),
                Expression::Access(AccessExpression::Member(second)),
            ) => {
                // Lookup the struct symbols associated with the expressions.
                // TODO: Remove clones
                let first_struct_symbol =
                    self.lookup_struct_symbol(&Expression::Access(AccessExpression::Member(first.clone())));
                let second_struct_symbol =
                    self.lookup_struct_symbol(&Expression::Access(AccessExpression::Member(second.clone())));

                match (first_struct_symbol, second_struct_symbol) {
                    (Some(first_struct_symbol), Some(second_struct_symbol)) => {
                        let first_member_struct = self.symbol_table.lookup_struct(first_struct_symbol).unwrap();
                        let second_member_struct = self.symbol_table.lookup_struct(second_struct_symbol).unwrap();
                        // Note that type checking guarantees that both expressions have the same same type. This is a sanity check.
                        assert_eq!(first_member_struct, second_member_struct);

                        // For each struct member, construct a new ternary expression.
                        let members = first_member_struct
                            .members
                            .iter()
                            .map(|Member { identifier, .. }| {
                                // Construct a new ternary expression for the struct member.
                                let (expression, stmts) = self.reconstruct_ternary(TernaryExpression {
                                    condition: input.condition.clone(),
                                    if_true: Box::new(Expression::Access(AccessExpression::Member(MemberAccess {
                                        inner: Box::new(Expression::Access(AccessExpression::Member(first.clone()))),
                                        name: *identifier,
                                        span: Default::default(),
                                    }))),
                                    if_false: Box::new(Expression::Access(AccessExpression::Member(MemberAccess {
                                        inner: Box::new(Expression::Access(AccessExpression::Member(second.clone()))),
                                        name: *identifier,
                                        span: Default::default(),
                                    }))),
                                    span: Default::default(),
                                });

                                // Accumulate any statements generated.
                                statements.extend(stmts);

                                // Create and accumulate an intermediate assignment statement for the ternary expression corresponding to the struct member.
                                let (identifier, statement) = self.unique_simple_assign_statement(expression);
                                statements.push(statement);

                                StructVariableInitializer {
                                    identifier,
                                    expression: Some(Expression::Identifier(identifier)),
                                }
                            })
                            .collect();

                        let (expr, stmts) = self.reconstruct_struct_init(StructExpression {
                            name: first_member_struct.identifier,
                            members,
                            span: Default::default(),
                        });

                        // Accumulate any statements generated.
                        statements.extend(stmts);

                        // Create a new assignment statement for the struct expression.
                        let (identifier, statement) = self.unique_simple_assign_statement(expr);

                        // Mark the lhs of the assignment as a struct.
                        self.structs
                            .insert(identifier.name, first_member_struct.identifier.name);

                        statements.push(statement);

                        (Expression::Identifier(identifier), statements)
                    }
                    _ => {
                        let if_true = Expression::Access(AccessExpression::Member(first));
                        let if_false = Expression::Access(AccessExpression::Member(second));
                        // Reconstruct the true case.
                        let (if_true, stmts) = self.reconstruct_expression(if_true);
                        statements.extend(stmts);

                        // Reconstruct the false case.
                        let (if_false, stmts) = self.reconstruct_expression(if_false);
                        statements.extend(stmts);

                        let (identifier, statement) =
                            self.unique_simple_assign_statement(Expression::Ternary(TernaryExpression {
                                condition: input.condition,
                                if_true: Box::new(if_true),
                                if_false: Box::new(if_false),
                                span: input.span,
                            }));

                        // Accumulate the new assignment statement.
                        statements.push(statement);

                        (Expression::Identifier(identifier), statements)
                    }
                }
            }
            // If both expressions are identifiers which are structs, construct ternary expression for each of the members and a struct expression for the result.
            (Expression::Identifier(first), Expression::Identifier(second))
                if self.structs.contains_key(&first.name) && self.structs.contains_key(&second.name) =>
            {
                let first_struct = self
                    .symbol_table
                    .lookup_struct(*self.structs.get(&first.name).unwrap())
                    .unwrap();
                let second_struct = self
                    .symbol_table
                    .lookup_struct(*self.structs.get(&second.name).unwrap())
                    .unwrap();
                // Note that type checking guarantees that both expressions have the same same type. This is a sanity check.
                assert_eq!(first_struct, second_struct);

                // For each struct member, construct a new ternary expression.
                let members = first_struct
                    .members
                    .iter()
                    .map(|Member { identifier, .. }| {
                        // Construct a new ternary expression for the struct member.
                        let (expression, stmts) = self.reconstruct_ternary(TernaryExpression {
                            condition: input.condition.clone(),
                            if_true: Box::new(Expression::Access(AccessExpression::Member(MemberAccess {
                                inner: Box::new(Expression::Identifier(first)),
                                name: *identifier,
                                span: Default::default(),
                            }))),
                            if_false: Box::new(Expression::Access(AccessExpression::Member(MemberAccess {
                                inner: Box::new(Expression::Identifier(second)),
                                name: *identifier,
                                span: Default::default(),
                            }))),
                            span: Default::default(),
                        });

                        // Accumulate any statements generated.
                        statements.extend(stmts);

                        // Create and accumulate an intermediate assignment statement for the ternary expression corresponding to the struct member.
                        let (identifier, statement) = self.unique_simple_assign_statement(expression);
                        statements.push(statement);

                        StructVariableInitializer {
                            identifier,
                            expression: Some(Expression::Identifier(identifier)),
                        }
                    })
                    .collect();

                let (expr, stmts) = self.reconstruct_struct_init(StructExpression {
                    name: first_struct.identifier,
                    members,
                    span: Default::default(),
                });

                // Accumulate any statements generated.
                statements.extend(stmts);

                // Create a new assignment statement for the struct expression.
                let (identifier, statement) = self.unique_simple_assign_statement(expr);

                // Mark the lhs of the assignment as a struct.
                self.structs.insert(identifier.name, first_struct.identifier.name);

                statements.push(statement);

                (Expression::Identifier(identifier), statements)
            }
            // Otherwise, create a new intermediate assignment for the ternary expression are return the assigned variable.
            // Note that a new assignment must be created to flattened nested ternary expressions.
            (if_true, if_false) => {
                // Reconstruct the true case.
                let (if_true, stmts) = self.reconstruct_expression(if_true);
                statements.extend(stmts);

                // Reconstruct the false case.
                let (if_false, stmts) = self.reconstruct_expression(if_false);
                statements.extend(stmts);

                let (identifier, statement) =
                    self.unique_simple_assign_statement(Expression::Ternary(TernaryExpression {
                        condition: input.condition,
                        if_true: Box::new(if_true),
                        if_false: Box::new(if_false),
                        span: input.span,
                    }));

                // Accumulate the new assignment statement.
                statements.push(statement);

                (Expression::Identifier(identifier), statements)
            }
        }
    }
}
