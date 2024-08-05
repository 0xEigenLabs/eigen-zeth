cd ..
echo $PWD
mkdir -p tmp/tools
cd tmp/tools
rm -rf ./*
git clone https://github.com/protolambda/eth2-val-tools.git
cd eth2-val-tools
git checkout 0d6d1ddb36479e73d7d876b29ac2d10ab3988e85
echo "Build eth2-val-tools, target path: $PWD/tools/bin/eth2-val-tools"
go build -o ../bin/eth2-val-tools