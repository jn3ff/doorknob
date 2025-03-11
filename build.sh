#!/bin/bash

echo "building for rpi"
cargo zigbuild --target aarch64-unknown-linux-gnu --release --features hardware
