set -e

#CURRENT_PATH=$(pwd)
CURRENT_PATH=$( cd "$( dirname "$0" )" && pwd )
PROJECT_PATH=$(dirname "${CURRENT_PATH}")
echo "PROJECT_PATH: ${PROJECT_PATH}"

GENESIS_PATH="${PROJECT_PATH}/testdata/layer2/pos/el-cl-genesis-data"
echo "GENESIS_PATH: ${GENESIS_PATH}"

echo "Removing existing genesis data: ${GENESIS_PATH}"
rm -rf ${GENESIS_PATH}

FILE_DIR="${PROJECT_PATH}/testdata/layer2/pos"
FILE_PATH="${FILE_DIR}/values.env"
echo "Updating GENESIS_TIMESTAMP in ${FILE_PATH}"

current_utc_timestamp=$(date -u +%s)
echo "current_utc_timestamp: ${current_utc_timestamp}"
OS="$(uname -s)"
case "${OS}" in
    Linux*)
        echo "Running on Linux"
        sed -i "s/GENESIS_TIMESTAMP=[0-9]*/GENESIS_TIMESTAMP=$current_utc_timestamp/" $FILE_PATH
        ;;
    Darwin*)
        echo "Running on macOS"
        sed -i '' "s/GENESIS_TIMESTAMP=[0-9]*/GENESIS_TIMESTAMP=$current_utc_timestamp/" $FILE_PATH
        ;;
    *)
        echo "Unknown OS: ${OS}"
        exit 1
        ;;
esac

# generate genesis data
echo "Generating genesis data: ${GENESIS_PATH}"
docker run --rm -it \
-v ${GENESIS_PATH}:/data \
-v ${FILE_PATH}:/config/values.env \
ethpandaops/ethereum-genesis-generator:3.2.1 all