#!/bin/bash
RUST_BACKTRACE=1 GENESIS_PERIOD=200 cargo test $1 --features=test-utilities
