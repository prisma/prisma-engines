set -euxo pipefail

echo 'Creating out dir...'
mkdir -p $out/src;

echo 'Copying package.json...'
cp ./prisma-fmt-wasm/package.json $out/;

echo 'Copying README.md...'
cp ./prisma-fmt-wasm/README.md $out/;

echo 'Generating node package...'
wasm-bindgen \
  --target nodejs \
  --out-dir $out/src \
  target/wasm32-unknown-unknown/release/prisma_fmt_build.wasm;
