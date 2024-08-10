set -e

CURRENT_PATH=$( cd "$( dirname "$0" )" && pwd )
PROJECT_PATH=$(dirname "${CURRENT_PATH}")
echo "PROJECT_PATH: ${PROJECT_PATH}"

LIGHTHOUSE_SOURCE_PATH="${PROJECT_PATH}/tmp/lighthouse-source"
echo "LIGHTHOUSE_SOURCE_PATH: ${LIGHTHOUSE_SOURCE_PATH}"

if [ ! -d "${LIGHTHOUSE_SOURCE_PATH}" ]; then
  mkdir -p "${LIGHTHOUSE_SOURCE_PATH}"
fi

cd ${LIGHTHOUSE_SOURCE_PATH}

echo "Removing existing lighthouse source"
rm -rf ./*

echo "Cloning lighthouse source"
git clone https://github.com/sigp/lighthouse.git
cd lighthouse

## lighthouse tag: v5.2.1
echo "Checkout lighthouse commit: 9e12c21f268c80a3f002ae0ca27477f9f512eb6f"
git checkout 9e12c21f268c80a3f002ae0ca27477f9f512eb6f

echo "Build and Install lighthouse"
make install