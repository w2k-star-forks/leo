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

use leo_span::Symbol;

use indexmap::IndexMap;

#[derive(Debug, Default)]
pub struct DeadCodeEliminator {
    /// A mapping determining which symbols are marked.
    marked: IndexMap<Symbol, bool>,
    /// A flag that determines if we are traversing a portion of the AST that has an effect on output.
    is_critical: bool,
}

impl DeadCodeEliminator {
    /// A function that returns whether or not a symbol is marked.
    /// If a symbol is marked, then it's declaration is not dead code.
    /// If a symbol is not marked, then it's declaration is dead code.
    pub(crate) fn is_marked(&self, symbol: &Symbol) -> bool {
        *self.marked.get(symbol).unwrap_or(&false)
    }

    /// A function that marks a symbol.
    pub(crate) fn mark(&mut self, symbol: Symbol) {
        self.marked.insert(symbol, true);
    }

    /// A function that sets the critical flag.
    pub(crate) fn set_critical(&mut self) {
        self.is_critical = true
    }

    /// A function that unsets the critical flag.
    pub(crate) fn unset_critical(&mut self) {
        self.is_critical = false
    }

    /// Returns the status of the critical flag.
    pub(crate) fn is_critical(&self) -> bool {
        self.is_critical
    }
}
