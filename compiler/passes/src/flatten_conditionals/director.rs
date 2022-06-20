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

use crate::FlattenConditionalStatements;
use leo_ast::{
    ExpressionReducerDirector, ProgramReducerDirector, ReducerDirector, StatementReducerDirector, TypeReducerDirector,
};

#[derive(Default)]
pub(crate) struct Director<'a> {
    reducer: FlattenConditionalStatements<'a>,
}

impl<'a> ReducerDirector for Director<'a> {
    type Reducer = FlattenConditionalStatements<'a>;

    fn reducer(self) -> Self::Reducer {
        self.reducer
    }

    fn reducer_ref(&mut self) -> &mut Self::Reducer {
        &mut self.reducer
    }
}

impl<'a> TypeReducerDirector for Director<'a> {}

impl<'a> ExpressionReducerDirector for Director<'a> {}

impl<'a> StatementReducerDirector for Director<'a> {}

impl<'a> ProgramReducerDirector for Director<'a> {}
