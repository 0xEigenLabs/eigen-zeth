set -e

#CURRENT_PATH=$(pwd)
CURRENT_PATH=$( cd "$( dirname "$0" )" && pwd )
PROJECT_PATH=$(dirname "${CURRENT_PATH}")
echo "PROJECT_PATH: ${PROJECT_PATH}"

GENESIS_PATH="${PROJECT_PATH}/testdata/layer2/pos/el-cl-genesis-data"
if [ ! -d "${GENESIS_PATH}" ]; then
    echo "Genesis directory not found at ${GENESIS_PATH}"
    exit 1
fi
echo "GENESIS_PATH: ${GENESIS_PATH}"

TESTNET_DIR="${GENESIS_PATH}/metadata"
echo "TESTNET_DIR: ${TESTNET_DIR}"

CHAIN_SPEC_PATH="${TESTNET_DIR}/genesis.json"
echo "CHAIN_SPEC_PATH: ${CHAIN_SPEC_PATH}"

CHAIN_DATA_PATH_PREFIX="${PROJECT_PATH}/tmp/layer2"
echo "CHAIN_DATA_PATH_PREFIX: ${CHAIN_DATA_PATH_PREFIX}"

EXECUTION_DATA_PATH="${CHAIN_DATA_PATH_PREFIX}/execution-data"
echo "EXECUTION_DATA_PATH: ${EXECUTION_DATA_PATH}"

CONSENSUS_DATA_PATH="${CHAIN_DATA_PATH_PREFIX}/consensus-data"
echo "CONSENSUS_DATA_PATH: ${CONSENSUS_DATA_PATH}"

VALIDATOR_BASE_PATH="${CONSENSUS_DATA_PATH}/validators"
echo "VALIDATOR_BASE_PATH: ${VALIDATOR_BASE_PATH}"
VALIDATOR_KEYS_PATH="${VALIDATOR_BASE_PATH}/keys"
VALIDATOR_SECRETS_PATH="${VALIDATOR_BASE_PATH}/secrets"

JWT_SECRET_PATH="${GENESIS_PATH}/jwt/jwtsecret"
echo "JWT_SECRET_PATH: ${JWT_SECRET_PATH}"

VALIDATOR_SOURCE_PATH="${PROJECT_PATH}/testdata/layer2/pos/validator-keys"

ZETH_LOG_FILE="${PROJECT_PATH}/tmp/zeth.log"
BEACON_LOG_FILE="${PROJECT_PATH}/tmp/beacon.log"
VALIDATOR_LOG_FILE="${PROJECT_PATH}/tmp/validator.log"

DEFAULT_SETTLEMENT="custom" # default settlement layer
DEFAULT_SETTLEMENT_CONFIG_FILE="${PROJECT_PATH}/configs/settlement.toml"
DEFAULT_CUSTOM_NODE_CONFIG_FILE="${PROJECT_PATH}/configs/custom_node_config.toml"
DEFAULT_DATABASE_CONFIG_FILE="${PROJECT_PATH}/configs/database.toml"

# start the zeth node
PROVER_ADDR=http://localhost:50061 RUST_LOG="debug,evm=trace,consensus::auto=trace,consensus::engine=trace,rpc::eth=trace" nohup eigen-zeth run --database mdbx --database-conf $DEFAULT_DATABASE_CONFIG_FILE --custom-node-conf $DEFAULT_CUSTOM_NODE_CONFIG_FILE --settlement $DEFAULT_SETTLEMENT --settlement-conf $DEFAULT_SETTLEMENT_CONFIG_FILE --chain $CHAIN_SPEC_PATH --datadir $EXECUTION_DATA_PATH --http --http.addr 0.0.0.0 --http.port 48546 --http.api debug,eth,net,trace,web3,rpc --authrpc.jwtsecret $JWT_SECRET_PATH --authrpc.addr 0.0.0.0 --authrpc.port 48552 --port 40304 --full > $ZETH_LOG_FILE 2>&1 &
# start the beacon node
nohup lighthouse bn --debug-level=info --datadir=$CONSENSUS_DATA_PATH --disable-enr-auto-update --enr-address=172.20.0.3 --enr-udp-port=9000 --enr-tcp-port=9000 --listen-address=0.0.0.0 --port=9000 --http --http-address=0.0.0.0 --http-port=4000 --slots-per-restore-point=32 --disable-packet-filter --execution-endpoints=http://localhost:48552 --jwt-secrets=$JWT_SECRET_PATH --suggested-fee-recipient=0x8943545177806ED17B9F23F0a21ee5948eCaa776 --subscribe-all-subnets --enable-private-discovery --testnet-dir=$TESTNET_DIR >> $BEACON_LOG_FILE 2>&1 &
# start the validator client
if [ ! -d "${VALIDATOR_BASE_PATH}" ]; then
    mkdir -p ${VALIDATOR_BASE_PATH}
fi

echo "Copying validator keys and secrets"
cp -rn "${VALIDATOR_SOURCE_PATH}/keys" ${VALIDATOR_BASE_PATH}
cp -rn "${VALIDATOR_SOURCE_PATH}/secrets" ${VALIDATOR_BASE_PATH}
nohup lighthouse vc --debug-level=info --testnet-dir=$TESTNET_DIR --validators-dir=$VALIDATOR_KEYS_PATH --secrets-dir=$VALIDATOR_SECRETS_PATH --init-slashing-protection --beacon-nodes=http://localhost:4000 --suggested-fee-recipient=0x8943545177806ED17B9F23F0a21ee5948eCaa776 >> $VALIDATOR_LOG_FILE 2>&1 &
