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

use crate::{TypeChecker, VariableSymbol, VariableType};

use leo_ast::*;
use leo_errors::TypeCheckerError;

use leo_span::sym;

use std::cell::RefCell;
use std::collections::HashSet;

impl<'a> ProgramVisitor<'a> for TypeChecker<'a> {
    fn visit_function(&mut self, input: &'a Function) {
        // Check that the function's annotations are valid.
        for annotation in input.annotations.iter() {
            match annotation.identifier.name {
                // Set `is_program_function` to true if the corresponding annotation is found.
                sym::program => self.is_program_function = true,
                sym::inline => self.is_inlined = true,
                _ => self.emit_err(TypeCheckerError::unknown_annotation(annotation, annotation.span)),
            }
        }
        if self.is_program_function && self.is_inlined {
            let mut spans = input.annotations.iter().map(|annotation| annotation.span);
            // This is safe, since if either `is_program_function` or `is_inlined` is true, then the function must have at least one annotation.
            let first_span = spans.next().unwrap();

            // Sum up the spans of all the annotations.
            let span = spans.fold(first_span, |acc, span| acc + span);
            self.emit_err(TypeCheckerError::program_and_inline_annotation(span));
        }

        let prev_st = std::mem::take(&mut self.symbol_table);
        self.symbol_table
            .swap(prev_st.borrow().lookup_fn_scope(input.name()).unwrap());
        self.symbol_table.borrow_mut().parent = Some(Box::new(prev_st.into_inner()));

        self.has_return = false;
        self.parent = Some(input.name());
        input.input.iter().for_each(|input_var| {
            self.assert_not_tuple(input_var.span, &input_var.type_);

            // If the function is not a program function, then check that the parameters do not have an associated mode.
            if !self.is_program_function && input_var.mode() != ParamMode::None {
                self.emit_err(TypeCheckerError::helper_function_inputs_cannot_have_modes(
                    input_var.span,
                ));
            }

            // Check for conflicting variable names.
            if let Err(err) = self.symbol_table.borrow_mut().insert_variable(
                input_var.identifier.name,
                VariableSymbol {
                    type_: input_var.type_.clone(),
                    span: input_var.identifier.span(),
                    declaration: VariableType::Input(input_var.mode()),
                },
            ) {
                self.handler.emit_err(err);
            }
        });
        self.visit_block(&input.block);

        if !self.has_return {
            self.emit_err(TypeCheckerError::function_has_no_return(input.name(), input.span()));
        }

        // Ensure there are no nested tuples in the return type.
        if let Type::Tuple(tys) = &input.output {
            for ty in &tys.0 {
                self.assert_not_tuple(input.span, ty);
            }
        }

        let prev_st = *self.symbol_table.borrow_mut().parent.take().unwrap();
        self.symbol_table.swap(prev_st.lookup_fn_scope(input.name()).unwrap());
        self.symbol_table = RefCell::new(prev_st);

        // Unset the flags associated with annotations.
        self.is_program_function = false;
        self.is_inlined = false;
    }

    fn visit_circuit(&mut self, input: &'a Circuit) {
        // Check for conflicting circuit/record member names.
        let mut used = HashSet::new();
        if !input.members.iter().all(|member| used.insert(member.name())) {
            self.emit_err(if input.is_record {
                TypeCheckerError::duplicate_record_variable(input.name(), input.span())
            } else {
                TypeCheckerError::duplicate_circuit_member(input.name(), input.span())
            });
        }

        // For records, enforce presence of `owner: Address` and `gates: u64` members.
        if input.is_record {
            let check_has_field = |need, expected_ty: Type| match input
                .members
                .iter()
                .find_map(|CircuitMember::CircuitVariable(v, t)| (v.name == need).then(|| (v, t)))
            {
                Some((_, actual_ty)) if expected_ty.eq_flat(actual_ty) => {} // All good, found + right type!
                Some((field, _)) => {
                    self.emit_err(TypeCheckerError::record_var_wrong_type(
                        field,
                        expected_ty,
                        input.span(),
                    ));
                }
                None => {
                    self.emit_err(TypeCheckerError::required_record_variable(
                        need,
                        expected_ty,
                        input.span(),
                    ));
                }
            };
            check_has_field(sym::owner, Type::Address);
            check_has_field(sym::gates, Type::U64);
        }

        // Ensure there are no tuple typed members.
        for CircuitMember::CircuitVariable(v, type_) in input.members.iter() {
            self.assert_not_tuple(v.span, type_);
        }
    }
}
