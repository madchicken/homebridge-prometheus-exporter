#!/usr/bin/env bash
rm -rf target
rm *.tgz
docker run --rm -v $(pwd):/app rust-compile-aarc64
docker run --rm -v $(pwd):/app rust-compile-armv7
cargo build --release

tar cvfz homebridge-exporter-aarc64.tgz target/aarch64-unknown-linux-gnu/release/homebridge-exporter
tar cvfz homebridge-exporter-armv7.tgz target/armv7-unknown-linux-gnueabihf/release/homebridge-exporter
tar cvfz homebridge-exporter-darwin-arm64.tgz target/release/homebridge-exporter