#!/bin/bash
export PKG_CONFIG_ALLOW_CROSS=1
export OPENSSL_STATIC=true
export OPENSSL_DIR=/musl
cargo build --target x86_64-unknown-linux-musl --release
strip target/x86_64-unknown-linux-musl/release/PatchyVideo-related-video-finder
docker build --no-cache -t patchyvideo-related-video-finder .
docker save -o patchyvideo-related-video-finder.tar patchyvideo-related-video-finder
