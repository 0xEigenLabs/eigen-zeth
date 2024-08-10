set -e

CURRENT_PATH=$( cd "$( dirname "$0" )" && pwd )
PROJECT_PATH=$(dirname "${CURRENT_PATH}")
cd "${PROJECT_PATH}/testdata/layer2/pos"

eigen-zeth init --datadir ../../../tmp/layer2/execution-data --chain el-cl-genesis-data/metadata/genesis.json