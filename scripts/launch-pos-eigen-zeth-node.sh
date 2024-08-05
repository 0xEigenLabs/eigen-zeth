cd ..
echo $PWD
# start the zeth node
PROVER_ADDR=http://localhost:50061 RUST_LOG="debug,evm=trace,consensus::auto=trace,consensus::engine=trace,rpc::eth=trace" nohup eigen-zeth run --database mdbx --chain testdata/layer2/pos/el-cl-genesis-data/metadata/genesis.json --datadir tmp/layer2/execution-data --http --http.addr 0.0.0.0 --http.port 48546 --http.api debug,eth,net,trace,web3,rpc --authrpc.jwtsecret testdata/layer2/pos/el-cl-genesis-data/jwt/jwtsecret --authrpc.addr 0.0.0.0 --authrpc.port 48552 --port 40304 --full > tmp/zeth.log 2>&1 &
# start the beacon node
nohup lighthouse bn --debug-level=info --datadir=tmp/layer2/consensus-data --disable-enr-auto-update --enr-address=172.20.0.3 --enr-udp-port=9000 --enr-tcp-port=9000 --listen-address=0.0.0.0 --port=9000 --http --http-address=0.0.0.0 --http-port=4000 --slots-per-restore-point=32 --disable-packet-filter --execution-endpoints=http://localhost:48552 --jwt-secrets=testdata/layer2/pos/el-cl-genesis-data/jwt/jwtsecret --suggested-fee-recipient=0x8943545177806ED17B9F23F0a21ee5948eCaa776 --subscribe-all-subnets --enable-private-discovery --testnet-dir=testdata/layer2/pos/el-cl-genesis-data/metadata >> tmp/beacon.log 2>&1 &
# start the validator client
mkdir -p tmp/layer2/consensus-data/validators
cp -rn testdata/layer2/pos/validator-keys/keys tmp/layer2/consensus-data/validators
cp -rn testdata/layer2/pos/validator-keys/secrets tmp/layer2/consensus-data/validators
nohup lighthouse vc --debug-level=info --testnet-dir=testdata/layer2/pos/el-cl-genesis-data/metadata --validators-dir=tmp/layer2/consensus-data/validators/keys --secrets-dir=tmp/layer2/consensus-data/validators/secrets --init-slashing-protection --beacon-nodes=http://localhost:4000 --suggested-fee-recipient=0x8943545177806ED17B9F23F0a21ee5948eCaa776 >> tmp/validator.log 2>&1 &
