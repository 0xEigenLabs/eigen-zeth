# eigen-zeth

* Init the layer 1 chain

```shell
rm -rf /tmp/layer1/chain
reth init --datadir /tmp/layer1/chain --chain testdata/layer1/chain.json
RUST_LOG="debug,evm=trace,consensus::auto=trace,consensus::engine=trace,rpc::eth=trace" reth node -d --chain testdata/layer1/chain.json --datadir /tmp/layer1/chain --auto-mine --http --http.port 8545 --http.api debug,eth,net,trace,web3,rpc
```

* Init the chain and run the RPC-only node

```
rm -rf /tmp/layer2/chain
reth init --datadir /tmp/layer2/chain --chain testdata/layer2/chain.json
RUST_LOG="debug,evm=trace,consensus::auto=trace,consensus::engine=trace,rpc::eth=trace" reth node -d --chain testdata/layer2/chain.json --datadir /tmp/layer2/chain --auto-mine --http --http.port 8546 --http.api debug,eth,net,trace,web3,rpc

RUST_LOG="rpc::eth=trace" ZETH_DB_PATH=/tmp/chain PROVER_ADDR=http://localhost:50061 ZETH_L2_ADDR=http://localhost:8546 HOST=0.0.0.0:8182 cargo run -r -- run --database mdbx
```


* Call custom method
```
curl -H "Content-Type: application/json" -X POST --data '{"jsonrpc":"2.0","method":"eigenrpc_customMethod","params":[],"id": 10}' 127.0.0.1:8545

curl -H "Content-Type: application/json" -X POST --data '{"jsonrpc":"2.0","method":"eigenrpc_getBlockByNumber","params":[0],"id": 10}' 127.0.0.1:8545
```

You can also use [cast](https://github.com/foundry-rs/foundry/releases).

```
cast rpc eigenrpc_customMethod

cast rpc eigenrpc_getBlockByNumber 0
```
