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

use leo_ast::{Expression, ExpressionReconstructor, Identifier};

impl ExpressionReconstructor for DeadCodeEliminator {
    type AdditionalOutput = ();

    /// This function reduces an `Identifier` expression and marks the associated symbol if necessary.
    fn reconstruct_identifier(&mut self, identifier: Identifier) -> (Expression, Self::AdditionalOutput) {
        // If we are in a critical component of the AST, then we should mark the symbol.
        if self.is_critical() {
            self.mark(identifier.name);
        }

        (
            Expression::Identifier(Identifier {
                name: identifier.name,
                span: identifier.span,
            }),
            Default::default(),
        )
    }
}
