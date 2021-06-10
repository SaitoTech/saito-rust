#!/bin/bash
EPOCH_LENGTH=100 cargo test $1 --features=test-utilities -- --nocapture