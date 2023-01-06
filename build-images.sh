#!/usr/bin/env bash

docker build . -f Dockerfile.aarc64 -t rust-compile-aarc64
docker build . -f Dockerfile.armv7 -t rust-compile-armv7