# Solana TPU Client Test

This is a simple test repo to test solana tpu client.
Please use private rpc without any rate limits to run this test.

Run arguments:
```
  -r, --rpc-url <RPC_URL>  [default: https://api.testnet.solana.com]
  -w, --ws-url <WS_URL>    [default: wss://api.testnet.solana.com]
  -p, --payer <PAYER>      [default: /home/galactus/.config/solana/id.json]
  -n <N>                   [default: 100]
  -e
```
Here `n` is to send `n` transactions per second to the cluster using tpu client.
`e` to enable confirmations, `e` should not be activated while profiling memory as it creates a vector to track all the signatures.