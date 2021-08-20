usage() {
    echo "Usage:"
    echo "     testnet run <idx> [chain]"
    echo "     testnet purge"
    echo "     testnet set-key <idx> <suri>"
    echo "     testnet echo-set-key <suri>"
    echo "     testnet rotate-key <idx>"
    exit 
}

run_node() {
    local node_args
    if [ $1 -eq 1 ]; then
        node_args="--node-key 0000000000000000000000000000000000000000000000000000000000000001"
    else
        node_args="--bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp"
    fi
    chain=${2:-testnet-local}
    ./target/release/deer-node --chain $chain \
    --base-path tmp/data/node$1 \
    --port $((30332 + $1)) \
    --ws-port $((9943 + $1)) \
    --rpc-port $((9932 + $1)) \
    --rpc-methods Unsafe \
    --unsafe-rpc-external \
    --rpc-cors all \
    --unsafe-ws-external \
    --wasm-execution compiled \
    --pruning archive \
    --validator ${node_args}
}

purge_data() {
    rm -rf tmp/data
}

generate_pubkey() {
    subkey inspect ${2:-} "$1" | grep "Public key (hex)" | awk '{ print $4 }'
}

set_key() {
    key_sr=$(generate_pubkey "$2")
    key_ed=$(generate_pubkey "$2" "--scheme ed25519")
    curl http://localhost:$((9932 + $1)) -H "Content-Type:application/json;charset=utf-8" -d \
        '{"jsonrpc":"2.0","id":1,"method":"author_insertKey","params":["babe","'"$2"'","'$key_sr'"]}'
    sleep 1
    curl http://localhost:$((9932 + $1)) -H "Content-Type:application/json;charset=utf-8" -d \
        '{"jsonrpc":"2.0","id":1,"method":"author_insertKey","params":["gran","'"$2"'","'$key_ed'"]}'
}

rotate_key() {
    curl http://localhost:$((9932 + $1)) -H "Content-Type:application/json;charset=utf-8" -d \
        '{"id":1, "jsonrpc":"2.0", "method": "author_rotateKeys", "params":[]}'
}

echo_set_key() {
    key_sr=$(generate_pubkey "$1")
    key_ed=$(generate_pubkey "$1" "--scheme ed25519")
    echo 'curl http://localhost:9933 -H "Content-Type:application/json;charset=utf-8" -d '"'"'{"jsonrpc":"2.0","id":1,"method":"author_insertKey","params":["babe","'"$1"'","'$key_sr'"]}'"'"
    echo 'curl http://localhost:9933 -H "Content-Type:application/json;charset=utf-8" -d '"'"'{"jsonrpc":"2.0","id":1,"method":"author_insertKey","params":["gran","'"$1"'","'$key_ed'"]}'"'"
}


case $1 in
    run)
        run_node $2 $3
        ;;
    purge)
        purge_data
        ;;
    set-key)
        set_key $2 "$3"
        ;;
    echo-set-key)
        echo_set_key "$2"
        ;;
    rotate-key)
        rotate_key $2
        ;;
    *)
        usage
        ;;
esac