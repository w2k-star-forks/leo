/*
namespace: Compile
expectation: Pass
input_file: inputs/div.in
*/

program test.aleo {    
    transition main(a: u128, b: u128, c: u128) -> bool {
        return a / b == c;
    }
}
