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

use crate::{CallType, DiGraph, FunctionSymbol, SymbolTable};

use leo_ast::{Identifier, Node, Type};
use leo_core::*;
use leo_errors::{emitter::Handler, TypeCheckerError, TypeCheckerWarning};
use leo_span::{Span, Symbol};

use itertools::Itertools;
use std::cell::RefCell;

pub struct TypeChecker<'a> {
    pub(crate) symbol_table: RefCell<SymbolTable>,
    pub(crate) handler: &'a Handler,
    pub(crate) parent: Option<Symbol>,
    pub(crate) has_return: bool,
    pub(crate) negate: bool,
    /// Are we traversing a function, if so, what is its call type?
    /// Is it a program function, helper function, or inlined function?
    pub(crate) function: Option<(Symbol, CallType)>,
    /// A directed graph describing the caller-callee relationships of the program.
    /// A node corresponds to a function.
    /// A directed edge of the form `a --> b` corresponds to an invocation of function `b` in the body of `a`.
    pub(crate) call_graph: DiGraph<Symbol>,
    /// A directed graph describing the composite type dependencies of the program.
    /// A node corresponds to named composite type, either a circuit or record.
    /// A directed edge of the form `a --> b` corresponds to a dependency of composite type `b` on composite type `a`.
    pub(crate) _type_graph: DiGraph<Symbol>,
}

const BOOLEAN_TYPE: Type = Type::Boolean;

const FIELD_TYPE: Type = Type::Field;

const GROUP_TYPE: Type = Type::Group;

const SCALAR_TYPE: Type = Type::Scalar;

const INT_TYPES: [Type; 10] = [
    Type::I8,
    Type::I16,
    Type::I32,
    Type::I64,
    Type::I128,
    Type::U8,
    Type::U16,
    Type::U32,
    Type::U64,
    Type::U128,
];

const SIGNED_INT_TYPES: [Type; 5] = [Type::I8, Type::I16, Type::I32, Type::I64, Type::I128];

const UNSIGNED_INT_TYPES: [Type; 5] = [Type::U8, Type::U16, Type::U32, Type::U64, Type::U128];

const MAGNITUDE_TYPES: [Type; 3] = [Type::U8, Type::U16, Type::U32];

impl<'a> TypeChecker<'a> {
    /// Returns a new type checker given a symbol table and error handler.
    pub fn new(symbol_table: SymbolTable, handler: &'a Handler) -> Self {
        let circuit_names = symbol_table.circuits.keys().copied().collect();

        let function_names = symbol_table
            .functions
            .iter()
            .filter_map(
                |(name, function_symbol): (&Symbol, &FunctionSymbol)| match function_symbol.call_type {
                    CallType::Program => Some(*name),
                    _ => None,
                },
            )
            .collect();

        Self {
            symbol_table: RefCell::new(symbol_table),
            handler,
            parent: None,
            has_return: false,
            negate: false,
            function: None,
            call_graph: DiGraph::new(function_names),
            // TODO: Fix
            _type_graph: DiGraph::new(circuit_names),
        }
    }

    /// Emits a type checker error.
    pub(crate) fn emit_err(&self, err: TypeCheckerError) {
        self.handler.emit_err(err);
    }

    /// Emits a type checker warning.
    pub(crate) fn emit_warning(&self, warning: TypeCheckerWarning) {
        self.handler.emit_warning(warning);
    }

    /// Emits an error to the handler if the given type is invalid.
    fn check_type(&self, is_valid: impl Fn(&Type) -> bool, error_string: String, type_: &Option<Type>, span: Span) {
        if let Some(type_) = type_ {
            if !is_valid(type_) {
                self.emit_err(TypeCheckerError::expected_one_type_of(error_string, type_, span));
            }
        }
    }

    /// Emits an error if the two given types are not equal.
    pub(crate) fn check_eq_types(&self, t1: &Option<Type>, t2: &Option<Type>, span: Span) {
        match (t1, t2) {
            (Some(t1), Some(t2)) if t1 != t2 => self.emit_err(TypeCheckerError::type_should_be(t1, t2, span)),
            (Some(type_), None) | (None, Some(type_)) => {
                self.emit_err(TypeCheckerError::type_should_be("no type", type_, span))
            }
            _ => {}
        }
    }

    /// Use this method when you know the actual type.
    /// Emits an error to the handler if the `actual` type is not equal to the `expected` type.
    pub(crate) fn assert_and_return_type(&self, actual: Type, expected: &Option<Type>, span: Span) -> Type {
        if let Some(expected) = expected {
            if !actual.eq_flat(expected) {
                self.emit_err(TypeCheckerError::type_should_be(actual.clone(), expected, span));
            }
        }

        actual
    }

    /// Emits an error to the error handler if the `actual` type is not equal to the `expected` type.
    pub(crate) fn assert_type(&self, actual: &Option<Type>, expected: &Type, span: Span) {
        self.check_type(
            |actual: &Type| actual.eq_flat(expected),
            expected.to_string(),
            actual,
            span,
        )
    }

    /// Emits an error to the error handler if the actual type is not equal to any of the expected types.
    pub(crate) fn assert_one_of_types(&self, actual: &Option<Type>, expected: &[Type], span: Span) {
        self.check_type(
            |actual: &Type| expected.iter().any(|t: &Type| t == actual),
            types_to_string(expected),
            actual,
            span,
        )
    }

    /// Emits an error to the handler if the given type is not a boolean.
    pub(crate) fn assert_bool_type(&self, type_: &Option<Type>, span: Span) {
        self.check_type(
            |type_: &Type| BOOLEAN_TYPE.eq(type_),
            BOOLEAN_TYPE.to_string(),
            type_,
            span,
        )
    }

    /// Emits an error to the handler if the given type is not a field.
    pub(crate) fn assert_field_type(&self, type_: &Option<Type>, span: Span) {
        self.check_type(|type_: &Type| FIELD_TYPE.eq(type_), FIELD_TYPE.to_string(), type_, span)
    }

    /// Emits an error to the handler if the given type is not a group.
    pub(crate) fn assert_group_type(&self, type_: &Option<Type>, span: Span) {
        self.check_type(|type_: &Type| GROUP_TYPE.eq(type_), GROUP_TYPE.to_string(), type_, span)
    }

    /// Emits an error to the handler if the given type is not a scalar.
    pub(crate) fn assert_scalar_type(&self, type_: &Option<Type>, span: Span) {
        self.check_type(
            |type_: &Type| SCALAR_TYPE.eq(type_),
            SCALAR_TYPE.to_string(),
            type_,
            span,
        )
    }

    /// Emits an error to the handler if the given type is not an integer.
    pub(crate) fn assert_int_type(&self, type_: &Option<Type>, span: Span) {
        self.check_type(
            |type_: &Type| INT_TYPES.contains(type_),
            types_to_string(&INT_TYPES),
            type_,
            span,
        )
    }

    /// Emits an error to the handler if the given type is not a signed integer.
    pub(crate) fn assert_signed_int_type(&self, type_: &Option<Type>, span: Span) {
        self.check_type(
            |type_: &Type| SIGNED_INT_TYPES.contains(type_),
            types_to_string(&SIGNED_INT_TYPES),
            type_,
            span,
        )
    }

    /// Emits an error to the handler if the given type is not an unsigned integer.
    pub(crate) fn assert_unsigned_int_type(&self, type_: &Option<Type>, span: Span) {
        self.check_type(
            |type_: &Type| UNSIGNED_INT_TYPES.contains(type_),
            types_to_string(&UNSIGNED_INT_TYPES),
            type_,
            span,
        )
    }

    /// Emits an error to the handler if the given type is not a magnitude (u8, u16, u32).
    pub(crate) fn assert_magnitude_type(&self, type_: &Option<Type>, span: Span) {
        self.check_type(
            |type_: &Type| MAGNITUDE_TYPES.contains(type_),
            types_to_string(&MAGNITUDE_TYPES),
            type_,
            span,
        )
    }

    /// Emits an error to the handler if the given type is not a boolean or an integer.
    pub(crate) fn assert_bool_int_type(&self, type_: &Option<Type>, span: Span) {
        self.check_type(
            |type_: &Type| BOOLEAN_TYPE.eq(type_) | INT_TYPES.contains(type_),
            format!("{}, {}", BOOLEAN_TYPE, types_to_string(&INT_TYPES)),
            type_,
            span,
        )
    }

    /// Emits an error to the handler if the given type is not a field or integer.
    pub(crate) fn assert_field_int_type(&self, type_: &Option<Type>, span: Span) {
        self.check_type(
            |type_: &Type| FIELD_TYPE.eq(type_) | INT_TYPES.contains(type_),
            format!("{}, {}", FIELD_TYPE, types_to_string(&INT_TYPES)),
            type_,
            span,
        )
    }

    /// Emits an error to the handler if the given type is not a field or group.
    pub(crate) fn assert_field_group_type(&self, type_: &Option<Type>, span: Span) {
        self.check_type(
            |type_: &Type| FIELD_TYPE.eq(type_) | GROUP_TYPE.eq(type_),
            format!("{}, {}", FIELD_TYPE, GROUP_TYPE),
            type_,
            span,
        )
    }

    /// Emits an error to the handler if the given type is not a field, group, or integer.
    pub(crate) fn assert_field_group_int_type(&self, type_: &Option<Type>, span: Span) {
        self.check_type(
            |type_: &Type| FIELD_TYPE.eq(type_) | GROUP_TYPE.eq(type_) | INT_TYPES.contains(type_),
            format!("{}, {}, {}", FIELD_TYPE, GROUP_TYPE, types_to_string(&INT_TYPES),),
            type_,
            span,
        )
    }

    /// Emits an error to the handler if the given type is not a field, group, or signed integer.
    pub(crate) fn assert_field_group_signed_int_type(&self, type_: &Option<Type>, span: Span) {
        self.check_type(
            |type_: &Type| FIELD_TYPE.eq(type_) | GROUP_TYPE.eq(type_) | SIGNED_INT_TYPES.contains(type_),
            format!("{}, {}, {}", FIELD_TYPE, GROUP_TYPE, types_to_string(&SIGNED_INT_TYPES),),
            type_,
            span,
        )
    }

    /// Emits an error to the handler if the given type is not a field, scalar, or integer.
    pub(crate) fn assert_field_scalar_int_type(&self, type_: &Option<Type>, span: Span) {
        self.check_type(
            |type_: &Type| FIELD_TYPE.eq(type_) | SCALAR_TYPE.eq(type_) | INT_TYPES.contains(type_),
            format!("{}, {}, {}", FIELD_TYPE, SCALAR_TYPE, types_to_string(&INT_TYPES),),
            type_,
            span,
        )
    }

    /// Emits an error to the handler if the given type is not a field, group, scalar or integer.
    pub(crate) fn assert_field_group_scalar_int_type(&self, type_: &Option<Type>, span: Span) {
        self.check_type(
            |type_: &Type| {
                FIELD_TYPE.eq(type_) | GROUP_TYPE.eq(type_) | SCALAR_TYPE.eq(type_) | INT_TYPES.contains(type_)
            },
            format!(
                "{}, {}, {}, {}",
                FIELD_TYPE,
                GROUP_TYPE,
                SCALAR_TYPE,
                types_to_string(&INT_TYPES),
            ),
            type_,
            span,
        )
    }

    /// Emits an error if the `circuit` is not a core library circuit.
    /// Emits an error if the `function` is not supported by the circuit.
    pub(crate) fn check_core_circuit_call(&self, circuit: &Type, function: &Identifier) -> Option<CoreInstruction> {
        if let Type::Identifier(ident) = circuit {
            // Lookup core circuit
            match CoreInstruction::from_symbols(ident.name, function.name) {
                None => {
                    // Not a core library circuit.
                    self.emit_err(TypeCheckerError::invalid_core_instruction(
                        &ident.name,
                        function.name,
                        ident.span(),
                    ));
                }
                Some(core_circuit) => return Some(core_circuit),
            }
        }
        None
    }

    /// Returns the `circuit` type and emits an error if the `expected` type does not match.
    pub(crate) fn check_expected_circuit(&mut self, circuit: Identifier, expected: &Option<Type>, span: Span) -> Type {
        if let Some(Type::Identifier(expected)) = expected {
            if !circuit.matches(expected) {
                self.emit_err(TypeCheckerError::type_should_be(circuit.name, expected.name, span));
            }
        }

        Type::Identifier(circuit)
    }

    /// Emits an error if the type is a tuple.
    pub(crate) fn assert_not_tuple(&self, span: Span, type_: &Type) {
        if matches!(type_, Type::Tuple(_)) {
            self.emit_err(TypeCheckerError::tuple_not_allowed(span))
        }
    }
}

fn types_to_string(types: &[Type]) -> String {
    types.iter().map(|type_| type_.to_string()).join(", ")
}
