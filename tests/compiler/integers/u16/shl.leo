/*
namespace: Compile
expectation: Pass
input_file: inputs/add.in
*/

program test.aleo {    
    transition main(a: u16, b: u16, c: u16) -> bool {
        return a << 2u8 == a << b && a << 2u32 == c;
    }
}
