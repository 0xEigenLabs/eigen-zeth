# eigen-zeth

* Init the chain and run the RPC-only node

```
rm -rf /tmp/chain
reth init --datadir /tmp/chain --chain testdata/chain.json

RUST_LOG="rpc::eth=trace" RETH_DB_PATH=/tmp/chain cargo run  -r
```


* Call custom method
```
curl -H "Content-Type: application/json" -X POST --data '{"jsonrpc":"2.0","method":"eigenrpc_customMethod","params":[],"id": 10}' 127.0.0.1:8545

curl -H "Content-Type: application/json" -X POST --data '{"jsonrpc":"2.0","method":"eigenrpc_getBlockByNumber","params":[0],"id": 10}' 127.0.0.1:8545
```

