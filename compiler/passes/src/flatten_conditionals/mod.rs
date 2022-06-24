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

pub mod flattener;
pub use flattener::*;

use crate::Pass;

use leo_ast::{Ast, ProgramReconstructor};
use leo_errors::Result;

impl Pass for ConditionalStatementFlattener {
    type Input = Ast;
    type Output = Result<Ast>;

    fn do_pass(ast: Self::Input) -> Self::Output {
        let mut reconstructor = ConditionalStatementFlattener::default();
        let program = reconstructor.reconstruct_program(ast.into_repr());

        Ok(Ast::new(program))
    }
}
