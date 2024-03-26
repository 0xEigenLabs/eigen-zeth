use zeth_primitives::block::Header;
/*
use zeth_lib::{
    builder::{BlockBuilderStrategy, EthereumStrategy},
    consts::ETH_MAINNET_CHAIN_SPEC,
    input::{BlockBuildInput, StateInput},
    EthereumTxEssence,
};


pub fn main() {
    // Read the input previous block and transaction data
    let input = BlockBuildInput {
        state_input: StateInput::<EthereumTxEssence> {
            parent_header: Default::default(),
            beneficiary: Default::default(),
            gas_limit: Default::default(),
            timestamp: Default::default(),
            extra_data: Default::default(),
            mix_hash: Default::default(),
            transactions: vec![],
            withdrawals: vec![],
        },
        parent_state_trie: Default::default(),
        parent_storage: Default::default(),
        contracts: vec![],
        ancestor_headers: vec![],
    };
    /*
    let _: BlockBuildInput<EthereumTxEssence> =
        bincode::deserialize(&bincode::serialize(&input).unwrap()).unwrap();
    */

    // Build the resulting block
    let mut output = EthereumStrategy::build_from(&ETH_MAINNET_CHAIN_SPEC, input)
        .expect("Failed to build the resulting block");
    // Abridge successful construction results
    if let Some(replaced_state) = output.replace_state_with_hash() {
        // Leak memory, save cycles
        core::mem::forget(replaced_state);
    }
    // Output the construction result
    //env::commit(&output);
    // Leak memory, save cycles
    core::mem::forget(output);
}
*/

pub fn main() {
    let header = Header::default();
    println!("{:?}", header);
}
