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

//! This file contains tools for benchmarking the Leo compiler and its stages.

use leo_compiler::Compiler;
use leo_errors::emitter::{Emitter, Handler};
use leo_span::{source_map::FileName, symbol::SESSION_GLOBALS};
use leo_test_framework::get_benches;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

/// An enum to represent the stage of the Compiler we are benchmarking.
enum BenchMode {
    /// Benchmarks parsing.
    Parse,
    /// Benchmarks symbol table generation.
    Symbol,
    /// Benchmarks type checking.
    Type,
    /// Benchmarks function inlining.
    Inline,
    /// Benchmarks loop unrolling.
    Unroll,
    /// Benchmarks static single assignment.
    Ssa,
    /// Benchmarks all the above stages.
    Full,
}

/// A dummy buffer emitter since we only test on valid programs.
struct BufEmitter;

impl Emitter for BufEmitter {
    fn emit_err(&mut self, _: leo_errors::LeoError) {}

    fn last_emitted_err_code(&self) -> Option<i32> {
        None
    }

    fn emit_warning(&mut self, _: leo_errors::LeoWarning) {}
}

impl BufEmitter {
    fn new_handler() -> Handler {
        Handler::new(Box::new(Self))
    }
}

/// The name of the test, and the test content.
#[derive(Clone)]
struct Sample {
    name: String,
    input: String,
}

/// A helper function to help create a Leo Compiler struct.
fn new_compiler(handler: &Handler) -> Compiler<'_> {
    Compiler::new(
        String::new(),
        String::new(),
        handler,
        PathBuf::from(String::new()),
        PathBuf::from(String::new()),
        None,
    )
}

impl Sample {
    /// Loads all the benchmark samples.
    /// Leverages the test-framework to grab all tests
    /// that are passing compiler tests or marked as benchmark tests.
    fn load_samples() -> Vec<Self> {
        get_benches()
            .into_iter()
            .map(|(name, input)| Self { name, input })
            .collect()
    }

    fn data(&self) -> (&str, FileName) {
        black_box((&self.input, FileName::Custom(String::new())))
    }

    fn bench(&self, c: &mut Criterion, mode: BenchMode) {
        match mode {
            BenchMode::Parse => self.bench_parse(c),
            BenchMode::Symbol => self.bench_symbol_table(c),
            BenchMode::Type => self.bench_type_checker(c),
            BenchMode::Inline => self.bench_function_inliner(c),
            BenchMode::Unroll => self.bench_loop_unroller(c),
            BenchMode::Ssa => self.bench_ssa(c),
            BenchMode::Full => self.bench_full(c),
        }
    }

    /// Benchmarks `logic(compiler)` where `compiler` is provided.
    fn bencher(&self, c: &mut Criterion, mode: &str, mut logic: impl FnMut(Compiler) -> Duration) {
        c.bench_function(&format!("{} {}", mode, self.name), |b| {
            // Iter custom is used so we can use custom timings around the compiler stages.
            // This way we can only time the necessary stage.
            b.iter_custom(|iters| {
                (0..iters)
                    .map(|_| SESSION_GLOBALS.set(&<_>::default(), || logic(new_compiler(&BufEmitter::new_handler()))))
                    .sum()
            });
        });
    }

    /// Benchmarks `logic(compiler)` where `compiler` is provided.
    /// Parsing has already been done.
    fn bencher_after_parse(&self, c: &mut Criterion, mode: &str, mut logic: impl FnMut(Compiler) -> Duration) {
        self.bencher(c, mode, |mut compiler| {
            let (input, name) = self.data();
            compiler
                .parse_program_from_string(input, name)
                .expect("Failed to parse program");
            logic(compiler)
        });
    }

    fn bench_parse(&self, c: &mut Criterion) {
        self.bencher(c, "parse", |mut compiler| {
            let (input, name) = self.data();
            let start = Instant::now();
            let out = compiler.parse_program_from_string(input, name);
            let time = start.elapsed();
            out.expect("Failed to parse program");
            time
        })
    }

    fn bench_symbol_table(&self, c: &mut Criterion) {
        self.bencher_after_parse(c, "symbol table pass", |compiler| {
            let start = Instant::now();
            let out = compiler.symbol_table_pass();
            let time = start.elapsed();
            out.expect("failed to generate symbol table");
            time
        });
    }

    fn bench_type_checker(&self, c: &mut Criterion) {
        self.bencher_after_parse(c, "type checker pass", |compiler| {
            let symbol_table = compiler.symbol_table_pass().expect("failed to generate symbol table");
            let start = Instant::now();
            let out = compiler.type_checker_pass(symbol_table);
            let time = start.elapsed();
            out.expect("failed to run type check pass");
            time
        });
    }

    fn bench_function_inliner(&self, c: &mut Criterion) {
        self.bencher_after_parse(c, "function inlining pass", |mut compiler| {
            let symbol_table = compiler.symbol_table_pass().expect("failed to generate symbol table");
            let symbol_table = compiler
                .type_checker_pass(symbol_table)
                .expect("failed to run type check pass");
            let start = Instant::now();
            let out = compiler.function_inlining_pass(symbol_table);
            let time = start.elapsed();
            out.expect("failed to run function inlining pass");
            time
        });
    }

    fn bench_loop_unroller(&self, c: &mut Criterion) {
        self.bencher_after_parse(c, "loop unrolling pass", |mut compiler| {
            let symbol_table = compiler.symbol_table_pass().expect("failed to generate symbol table");
            let symbol_table = compiler
                .type_checker_pass(symbol_table)
                .expect("failed to run type check pass");
            let symbol_table = compiler
                .function_inlining_pass(symbol_table)
                .expect("failed to run function inlining pass");
            let start = Instant::now();
            let out = compiler.loop_unrolling_pass(symbol_table);
            let time = start.elapsed();
            out.expect("failed to run loop unrolling pass");
            time
        });
    }

    fn bench_ssa(&self, c: &mut Criterion) {
        self.bencher_after_parse(c, "full", |mut compiler| {
            let symbol_table = compiler.symbol_table_pass().expect("failed to generate symbol table");
            let symbol_table = compiler
                .type_checker_pass(symbol_table)
                .expect("failed to run type check pass");
            let symbol_table = compiler
                .function_inlining_pass(symbol_table)
                .expect("failed to run function inlining pass");
            compiler
                .loop_unrolling_pass(symbol_table)
                .expect("failed to run loop unrolling pass");
            let start = Instant::now();
            let out = compiler.static_single_assignment_pass();
            let time = start.elapsed();
            out.expect("failed to run ssa pass");
            time
        })
    }

    fn bench_full(&self, c: &mut Criterion) {
        self.bencher(c, "full", |mut compiler| {
            let (input, name) = self.data();
            let start = Instant::now();
            compiler
                .parse_program_from_string(input, name)
                .expect("Failed to parse program");
            let symbol_table = compiler.symbol_table_pass().expect("failed to generate symbol table");
            let symbol_table = compiler
                .type_checker_pass(symbol_table)
                .expect("failed to run type check pass");
            let symbol_table = compiler
                .function_inlining_pass(symbol_table)
                .expect("failed to run function inlining pass");
            compiler
                .loop_unrolling_pass(symbol_table)
                .expect("failed to run loop unrolling pass");
            compiler
                .static_single_assignment_pass()
                .expect("failed to run ssa pass");
            start.elapsed()
        })
    }
}

macro_rules! bench {
    ($name:ident, $mode:expr) => {
        fn $name(c: &mut Criterion) {
            Sample::load_samples().into_iter().for_each(|s| s.bench(c, $mode))
        }
    };
}

bench!(bench_parse, BenchMode::Parse);
bench!(bench_symbol, BenchMode::Symbol);
bench!(bench_type, BenchMode::Type);
bench!(bench_inline, BenchMode::Inline);
bench!(bench_unroll, BenchMode::Unroll);
bench!(bench_ssa, BenchMode::Ssa);
bench!(bench_full, BenchMode::Full);

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(200).measurement_time(Duration::from_secs(10)).nresamples(200_000);
    targets =
        bench_parse,
        bench_symbol,
        bench_type,
        bench_inline,
        bench_unroll,
        bench_ssa,
        bench_full
);
criterion_main!(benches);
