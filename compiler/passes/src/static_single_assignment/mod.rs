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

mod director;
use director::*;

pub mod reducer;
pub use reducer::*;

mod rename_table;
pub(crate) use rename_table::*;

use crate::Pass;

use leo_ast::{Ast, ProgramReducerDirector};
use leo_errors::{emitter::Handler, Result};

impl<'a> Pass for StaticSingleAssignmentReducer<'a> {
    type Input = (&'a Ast, &'a Handler);
    type Output = Result<Ast>;

    fn do_pass((ast, handler): Self::Input) -> Self::Output {
        let mut visitor = Director::new(handler);
        let program = visitor.reduce_program(ast.as_repr())?;
        handler.last_err()?;

        Ok(Ast::new(program))
    }
}
