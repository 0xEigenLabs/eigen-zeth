echo "Initializing layer2 chain with eigen-zeth/testdata/layer2/chain.json file, data directory: eigen-zeth/tmp/layer2/chain"
cd ..
cargo run -r -- init --datadir tmp/layer2/chain --chain testdata/layer2/chain.json