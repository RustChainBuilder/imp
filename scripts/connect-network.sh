#!/bin/bash
# sh connect-network.sh altona

NETWORK=$1
DATA_DIR=$HOME/.imp/datadir
LISTEN_ADDRESS=$(ipconfig getifaddr en0)
BOOTNODES=$(cat ~/.$NETWORK/enrs.txt | tr "\n" "," | sed -e "s/,$//g")
mkdir -p $DATA_DIR
RUST_BACKTRACE=full ./../target/debug/imp --p2p-protocol-version Lighthouse/v0.1.2-unstable --debug-level info mothra --debug-level info --boot-nodes $BOOTNODES --listen-address $LISTEN_ADDRESS --port 13001 --discovery-port 12001 --datadir $DATA_DIR
rm -rf $DATA_DIR