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

/// A `NameTable` is a lookup table for renamed variables within a single-basic block of the control-flow-graph.
/// While similar in functionality to `SymbolTable`, `NameTable` expresses parent relationships based on control flow whereas `SymbolTable` expresses them based on scope.
// Developer Note:
//   - The design of this struct relies on the assumption that the only control structures in the AST (when converting to SSA) are `ConditionalStatement`s.
//   - Consequently, the control-flow-graph is a tree. If this assumption changes, `NameTable` may need to be redesigned.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NameTable {
    /// Basic blocks that control-flow must pass through before reaching this one.
    parents: Vec<Box<NameTable>>,
    /// A mapping between symbols in the source code and the most recent name assigned to it by static single assignment.
    renamed_variables: IndexMap<Symbol, Symbol>,
}

impl NameTable {
    fn new(parents: Vec<Box<NameTable>>) -> Self {
        Self {
            parents,
            renamed_variables: IndexMap::new(),
        }
    }

    /// Adds a parent to this `NameTable`.
    /// Note that `parent` must correspond to a parent node in the control-flow-graph.
    fn add_parent(&mut self, parent: Box<NameTable>) {
        self.parents.push(parent);
    }

    /// If `old_symbol` is present in `renamed_variables` then it is replaced with `new_symbol`.
    /// Otherwise, create a new entry in `renamed_variables`.
    fn update(&mut self, old_symbol: Symbol, new_symbol: Symbol) {
        self.renamed_variables.insert(old_symbol, new_symbol);
    }

    /// Returns the names that have most recently been assigned to `symbol` at this point in the control-flow-graph.
    /// If `symbol` has an entry in `renamed_variables`, then it is returned.
    /// Otherwise, recursively search through the parent tables.
    fn lookup_variable(&self, symbol: &Symbol) -> Vec<Symbol> {
        let mut names = Vec::new();
        match self.renamed_variables.get(symbol) {
            Some(name) => names.push(name.clone()),
            None => {
                for parent in &self.parents {
                    names.append(&mut parent.lookup_variable(symbol));
                }
            }
        }
        names
    }
}
