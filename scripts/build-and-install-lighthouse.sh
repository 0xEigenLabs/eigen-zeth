cd ../testdata/layer2/pos/lighthouse-source
echo $PWD
rm -rf lighthouse
git clone https://github.com/sigp/lighthouse.git
cd lighthouse
echo "Build and Install lighthouse"
make install