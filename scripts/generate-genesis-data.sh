cd ../testdata/layer2/pos
rm -rf el-cl-genesis-data

file_path="values.env"
current_utc_timestamp=$(date -u +%s)
OS="$(uname -s)"
case "${OS}" in
    Linux*)
        echo "Running on Linux"
        sed -i "s/GENESIS_TIMESTAMP=[0-9]*/GENESIS_TIMESTAMP=$current_utc_timestamp/" $file_path
        ;;
    Darwin*)
        echo "Running on macOS"
        sed -i '' "s/GENESIS_TIMESTAMP=[0-9]*/GENESIS_TIMESTAMP=$current_utc_timestamp/" $file_path
        ;;
    *)
        echo "Unknown OS: ${OS}"
        exit 1
        ;;
esac

# generate genesis data
docker run --rm -it \
-v $PWD/el-cl-genesis-data:/data \
-v $PWD/values.env:/config/values.env \
ethpandaops/ethereum-genesis-generator:3.2.1 all