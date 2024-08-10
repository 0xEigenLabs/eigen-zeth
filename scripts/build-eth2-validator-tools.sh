set -e

CURRENT_PATH=$( cd "$( dirname "$0" )" && pwd )
PROJECT_PATH=$(dirname "${CURRENT_PATH}")
echo "PROJECT_PATH: ${PROJECT_PATH}"

INSTALL_PATH="${PROJECT_PATH}/tmp/tools/bin"
echo "Tools INSTALL_PATH: ${INSTALL_PATH}"
if [ ! -d "${PROJECT_PATH}/tmp/tools" ]; then
  mkdir -p "${PROJECT_PATH}/tmp/tools"
fi

cd "${PROJECT_PATH}/tmp/tools"

echo "Removing existing eth2-val-tools"
rm -rf ./*

echo "Cloning eth2-val-tools"
git clone https://github.com/protolambda/eth2-val-tools.git
cd eth2-val-tools

echo "Checkout eth2-val-tools commit: 0d6d1ddb36479e73d7d876b29ac2d10ab3988e85"
git checkout 0d6d1ddb36479e73d7d876b29ac2d10ab3988e85

echo "Build eth2-val-tools, target path: ${INSTALL_PATH}"
go build -o ${INSTALL_PATH}/eth2-val-tools