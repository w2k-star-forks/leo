/*
namespace: Compile
expectation: Fail
*/

program test.aleo {    
    record Token {
        // The token owner.
        owner: address,
        // The Aleo balance (in gates).
        gates: u64,
        // The token amount.
        amount: u64,
    }
    
    function mint(r0: address, r1: u64) -> Token {
        return Token {
            owner: r1, // This variable should be type address.
            gates: 0u64,
            amount: r0, // This variable should be type u64.
        };
    }
    
    function main(x: address) -> u64 {
        const c: u64 = 1u64;
        let t: Token = mint(x, c);
    
        return t.gates;
    }}
