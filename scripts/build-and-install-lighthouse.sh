cd ..
echo $PWD
mkdir -p tmp/lighthouse-source
cd tmp/lighthouse-source
rm -rf ./*
git clone https://github.com/sigp/lighthouse.git
cd lighthouse
## lighthouse tag: v5.2.1
git checkout 9e12c21f268c80a3f002ae0ca27477f9f512eb6f
echo "Build and Install lighthouse"
make install