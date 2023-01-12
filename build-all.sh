#!/usr/bin/env bash

docker run --rm -v $(pwd):/app rust-compile-aarc64
docker run --rm -v $(pwd):/app rust-compile-armv7
