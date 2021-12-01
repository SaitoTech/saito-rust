#!/bin/bash
RUST_LOG=trace cargo test $1 -- --nocapture
