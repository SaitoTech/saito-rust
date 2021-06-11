#!/bin/bash
RUST_BACKTRACE=1 EPOCH_LENGTH=200 cargo test $1 --features=test-utilities
