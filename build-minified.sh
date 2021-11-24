#!/bin/sh

TARGET=target/release/skip-deploy
cargo build --release && strip $TARGET && ls -lh $TARGET