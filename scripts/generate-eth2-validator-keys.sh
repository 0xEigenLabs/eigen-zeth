set -e

CURRENT_PATH=$( cd "$( dirname "$0" )" && pwd )
PROJECT_PATH=$(dirname "${CURRENT_PATH}")
echo "PROJECT_PATH: ${PROJECT_PATH}"

VALIDATOR_KEYS_PATH="${PROJECT_PATH}/testdata/layer2/pos/validator-keys"
echo "VALIDATOR_KEYS_PATH: ${VALIDATOR_KEYS_PATH}"

echo "Removing existing validator keys"
rm -rf ${VALIDATOR_KEYS_PATH}

DEFAULT_MNEMONIC="giant issue aisle success illegal bike spike question tent bar rely arctic volcano long crawl hungry vocal artwork sniff fantasy very lucky have athlete"
echo "Default mnemonic: ${DEFAULT_MNEMONIC}"

DEFAULT_TOOLS_BIN_PATH="${PROJECT_PATH}/tmp/tools/bin/eth2-val-tools"
echo "DEFAULT_TOOLS_BIN_PATH: ${DEFAULT_TOOLS_BIN_PATH}"
if [ ! -f "${DEFAULT_TOOLS_BIN_PATH}" ]; then
  echo "eth2-val-tools binary not found: ${DEFAULT_TOOLS_BIN_PATH}, please run make build_eth2_validator_tools or scripts/build-eth2-validator-tools.sh"
  exit 1
fi

${DEFAULT_TOOLS_BIN_PATH} keystores \
--insecure \
--prysm-pass password \
--out-loc ${VALIDATOR_KEYS_PATH} \
--source-mnemonic "${DEFAULT_MNEMONIC}" \
--source-min 0 \
--source-max 64