#!/usr/bin/env bash
rm -rf target/coverage || true
mkdir -p target/coverage

echo "Running tests"
RUST_LOG=mpvipc=trace RUSTFLAGS="-Cinstrument-coverage" LLVM_PROFILE_FILE="target/coverage/%p-%m.profraw" cargo nextest run --all-features --release --no-fail-fast

echo "Generating coverage report"
grcov \
  --source-dir . \
  --binary-path ./target/release/deps/ \
  --excl-start 'mod test* \{' \
  --ignore 'tests/*' \
  --ignore "*test.rs" \
  --ignore "*tests.rs" \
  --ignore "*github.com*" \
  --ignore "*libcore*" \
  --ignore "*rustc*" \
  --ignore "*liballoc*" \
  --ignore "*cargo*" \
  -t html \
  -o ./target/coverage/html \
  target/coverage/
