/*
namespace: Compile
expectation: Pass
input_file: inputs/div.in
*/

program test.aleo {    
    transition main(a: u64, b: u64, c: u64) -> bool {
        return a % b == c;
    }
}
