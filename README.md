# eigen-zeth

* Init the layer 1 chain

```shell
rm -rf /tmp/layer1/chain
reth init --datadir /tmp/layer1/chain --chain testdata/layer1/chain.json
RUST_LOG="debug,evm=trace,consensus::auto=trace,consensus::engine=trace,rpc::eth=trace" nohup reth node -d --chain testdata/layer1/chain.json --datadir tmp/layer1/chain --auto-mine --http --http.port 8547 --http.api debug,eth,net,trace,web3,rpc --port 30305 --authrpc.port 8553 > chain_layer1.log 2>&1 &
```

* Init the layer2 chain and rollup service

```
rm -rf /tmp/layer2/chain
cargo run -r -- init --datadir /tmp/layer2/chain --chain testdata/layer2/chain.json
PROVER_ADDR=http://localhost:50061 RUST_LOG="debug,evm=trace,consensus::auto=trace,consensus::engine=trace,rpc::eth=trace" nohup cargo run -r -- run --database mdbx --log-level debug --chain testdata/layer2/chain.json --http --http.port 8546 --http.api debug,eth,net,trace,web3,rpc --authrpc.port 8552 --port 30304 --datadir tmp/layer2/chain --auto-mine > chain_layer2.log 2>&1 &
```


* Call custom method
```
curl -H "Content-Type: application/json" -X POST --data '{"jsonrpc":"2.0","method":"eigenrpc_customMethod","params":[],"id": 10}' 127.0.0.1:8546

curl -H "Content-Type: application/json" -X POST --data '{"jsonrpc":"2.0","method":"eigenrpc_getBlockByNumber","params":[0],"id": 10}' 127.0.0.1:8546

curl -H "Content-Type: application/json" -X POST --data '{"jsonrpc":"2.0","method":"eigenrpc_getBatchProof","params":[0],"id": 10}' 127.0.0.1:8546
```

You can also use [cast](https://github.com/foundry-rs/foundry/releases).

```
cast rpc --rpc-url http://localhost:8546 eigenrpc_customMethod

cast rpc --rpc-url http://localhost:8546 eigenrpc_getBlockByNumber 0

cast rpc --rpc-url http://localhost:8546 eigenrpc_getBatchProof 0
```
