cd ..
echo $PWD
mkdir -p tmp/lighthouse-source
cd tmp/lighthouse-source
rm -rf ./*
git clone https://github.com/sigp/lighthouse.git
cd lighthouse
echo "Build and Install lighthouse"
make install