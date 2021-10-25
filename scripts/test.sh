#!/bin/bash
cargo test $1 -- --nocapture --test-threads=1
