#!/usr/bin/env bash
# Proves the license gate bites: add a GPL/LGPL crate, expect `cargo deny check licenses` to fail.
set -euo pipefail
cp crates/ark-core/Cargo.toml /tmp/ark-core.Cargo.toml.bak
cp Cargo.lock /tmp/Cargo.lock.bak 2>/dev/null || true
restore() { cp /tmp/ark-core.Cargo.toml.bak crates/ark-core/Cargo.toml; cp /tmp/Cargo.lock.bak Cargo.lock 2>/dev/null || true; }
trap restore EXIT
cargo add --quiet -p ark-core rug@1 --no-default-features
if cargo deny check licenses >/dev/null 2>&1; then
  echo "FAIL: cargo deny accepted an LGPL dependency (rug)" >&2
  exit 1
fi
echo "OK: cargo deny rejected the LGPL dependency"
