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

extern crate snarkvm;

use snarkvm::compiler::Program;
use snarkvm::prelude::Testnet3;

use std::{path::PathBuf, sync::Arc};
use std::fmt::Debug;
use std::str::FromStr;

type CurrentNetwork = Testnet3;

/// Returns the path to the `resources` folder for this module.
fn resources_path() -> PathBuf {
    // Construct the path for the `resources` folder.
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("src");
    path.push("resources");

    // Create the `resources` folder, if it does not exist.
    if !path.exists() {
        std::fs::create_dir_all(&path).unwrap_or_else(|_| panic!("Failed to create resources folder: {:?}", path));
    }
    // Output the path.
    path
}

/// Loads the given `test_folder/test_file` and asserts the given `candidate` matches the expected values.
#[track_caller]
fn assert_snapshot<S1: Into<String>, S2: Into<String>, C: Debug>(test_folder: S1, test_file: S2, candidate: C) {
    // Construct the path for the test folder.
    let mut path = resources_path();
    path.push(test_folder.into());

    // Create the test folder, if it does not exist.
    if !path.exists() {
        std::fs::create_dir(&path).unwrap_or_else(|_| panic!("Failed to create test folder: {:?}", path));
    }

    // Construct the path for the test file.
    path.push(test_file.into());
    path.set_extension("snap");

    // Create the test file, if it does not exist.
    if !path.exists() {
        std::fs::File::create(&path).unwrap_or_else(|_| panic!("Failed to create file: {:?}", path));
    }

    // Assert the test file is equal to the expected value.
    expect_test::expect_file![path].assert_eq(&format!("{:?}", candidate));
}

fn new_compiler(handler: &Handler, main_file_path: PathBuf) -> Compiler<'_> {
    let output_dir = PathBuf::from("/tmp/output/");
    std::fs::create_dir_all(output_dir.clone()).unwrap();

    Compiler::new(
        String::from("test"),
        String::from("aleo"),
        handler,
        main_file_path,
        output_dir,
        Some(OutputOptions {
            spans_enabled: false,
            initial_input_ast: true,
            initial_ast: true,
            unrolled_ast: true,
            ssa_ast: true,
        }),
    )
}

fn parse_program<'a>(
    handler: &'a Handler,
    program_string: &str,
    cwd: Option<PathBuf>,
) -> Result<Compiler<'a>, LeoError> {
    let mut compiler = new_compiler(handler, cwd.clone().unwrap_or_else(|| "compiler-test".into()));
    let name = cwd.map_or_else(|| FileName::Custom("compiler-test".into()), FileName::Real);
    compiler.parse_program_from_string(program_string, name)?;

    Ok(compiler)
}

fn compile_and_process<'a>(parsed: &'a mut Compiler<'a>) -> Result<SymbolTable, LeoError> {
    let st = parsed.symbol_table_pass()?;
    let st = parsed.type_checker_pass(st)?;
    let st = parsed.loop_unrolling_pass(st)?;
    parsed.static_single_assignment_pass()?;
    Ok(st)
}

#[test]
fn test_add() {
    let expected = r"program to_parse.aleo;

interface message:
    first as field;
    second as field;

function compute:
    input r0 as message.private;
    add r0.first r0.second into r1;
    output r1 as field.private;
";
    // Parse a new program.
    let aleo_program = Program::<CurrentNetwork>::from_str(expected)?;

    // Leo program -> aleo instructions.
    {
        todo!()
    }

    assert_snapshot("operators", "add", aleo_program);
    assert_snapshot("operators", "add", leo_program);
}