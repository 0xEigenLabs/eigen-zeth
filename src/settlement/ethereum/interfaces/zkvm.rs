//! Rust contract client for https://github.com/0xEigenLabs/eigen-bridge-contracts/blob/feature/bridge_contract/src/EigenZkVM.sol

// need abi.json
// abigen!(
//     EigenZkVM,
//     r#"[
//         function sequenceBatches(BatchData[] calldata batches,address l2Coinbase) external onlyAdmin
//     ]"#,
// );
//
// function verifyBatches(uint64 pendingStateNum, uint64 initNumBatch, uint64 finalNewBatch, bytes32 newLocalExitRoot, bytes32 newStateRoot, Proof calldata proof, uint[1] calldata input) external
// function verifyBatchesTrustedAggregator(uint64 pendingStateNum, uint64 initNumBatch, uint64 finalNewBatch, bytes32 newLocalExitRoot, bytes32 newStateRoot, Proof calldata proof, uint[1] calldata input) external onlyAdmin
