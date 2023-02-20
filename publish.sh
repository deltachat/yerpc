#!/bin/bash
set -e
dry="--dry-run"
if [ "$1" == "publish" ]; then
  dry=""
else
  echo "Pass \"publish\" as first argument to disable dry-run mode and publish for real."
fi

set -v
cargo build
cargo test
cargo test --all-features
cargo publish -p yerpc_derive $dry
cargo publish -p yerpc $dry
cargo publish -p yerpc-tide $dry
cd typescript
npm run clean
npm run lint
npm run build
npm publish "$dry"
