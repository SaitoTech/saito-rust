#!/usr/bin/env bash
curl 127.0.0.1:3030/handshakeinit\?a=02579d6ff84f661297f38e3eb20953824cfc279fee903a746b3dccb534677fd81a > test_challenge
cargo run --bin walletcli sign test_challenge signed_challenge
curl --data-binary "@signed_challenge" -X POST http://127.0.0.1:3030/handshakecomplete > ws_token
TOKEN="`cat ws_token`"
websocat ws://127.0.0.1:3030/wsconnect -H socket-token:$TOKEN -b readfile:test_challenge
