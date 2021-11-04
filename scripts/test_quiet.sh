#!/bin/bash
RUST_LOG=error cargo test $1 -- --nocapture --test-threads=1
