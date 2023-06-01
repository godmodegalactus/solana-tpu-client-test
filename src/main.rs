use rand::{distributions::Alphanumeric, prelude::Distribution, SeedableRng};
use solana_client::{
    connection_cache, nonblocking::tpu_client::TpuClient, tpu_client::TpuClientConfig,
};
use solana_sdk::{
    hash::Hash,
    instruction::Instruction,
    message::Message,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::Transaction,
};
use std::{str::FromStr, sync::Arc, time::Duration};
use tokio::sync::Mutex;

const MEMO_PROGRAM_ID: &str = "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr";

fn create_memo_tx(msg: &[u8], payer: &Keypair, blockhash: Hash) -> Transaction {
    let memo = Pubkey::from_str(MEMO_PROGRAM_ID).unwrap();

    let instruction = Instruction::new_with_bytes(memo, msg, vec![]);
    let message = Message::new(&[instruction], Some(&payer.pubkey()));
    Transaction::new(&[payer], message, blockhash)
}

#[tokio::main(flavor = "multi_thread", worker_threads = 16)]
pub async fn main() {
    let rpc_url = "https://api.testnet.solana.com";
    let ws_url = "wss://api.testnet.solana.com";
    let payer_location = "~/.config/solana/id.json";
    let enable_confirmation = false;

    let payer_file = tokio::fs::read_to_string(payer_location)
        .await
        .expect("Cannot find the payer keypair file");
    let payer_bytes: Vec<u8> = serde_json::from_str(&payer_file).unwrap();
    let payer = Keypair::from_bytes(payer_bytes.as_slice()).unwrap();

    let rpc_client = Arc::new(solana_client::nonblocking::rpc_client::RpcClient::new(
        rpc_url.to_string(),
    ));

    let connection_cache = Arc::new(connection_cache::ConnectionCache::new(4));
    let tpu_client = TpuClient::new_with_connection_cache(
        rpc_client.clone(),
        ws_url,
        TpuClientConfig {
            ..Default::default()
        },
        connection_cache,
    )
    .await
    .unwrap();

    let txs_sent_deque = Arc::new(Mutex::new(Vec::<Vec<Signature>>::new()));
    // creating confirming task
    if enable_confirmation {
        let rpc_client = rpc_client.clone();
        let txs_sent_deque = txs_sent_deque.clone();
        tokio::spawn(async move {
            let mut counter = 0;
            // wait max of 2 minutes to validate a batch
            let maximum_wait_duration = Duration::from_secs(120);
            loop {
                let mut queue = {
                    let mut lk = txs_sent_deque.lock().await;
                    let tasks = lk.clone();
                    lk.clear();
                    tasks
                };
                while !queue.is_empty() {
                    let mut txs = queue.pop().unwrap();
                    let start = tokio::time::Instant::now();
                    let mut confirmed = 0;
                    while !txs.is_empty() && start.elapsed() < maximum_wait_duration {
                        if let Ok(status) = rpc_client.get_signature_statuses(txs.as_slice()).await
                        {
                            let len = status.value.len();
                            for i in (0..len).rev() {
                                if let Some(_) = &status.value[i] {
                                    confirmed += 1;
                                    txs.remove(i);
                                }
                            }
                        }
                    }
                    println!(
                        "For batch {} : {} of 10 transactions were confirmed",
                        counter, confirmed
                    );
                    counter += 1;
                }
            }
        });
    }

    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let blockhash = if let Ok(bh) = rpc_client.get_latest_blockhash().await {
            bh
        } else {
            println!("error fetching blockhash");
            continue;
        };

        let mut txs = vec![];
        for seed in 0..10 {
            let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
            let msg: Vec<u8> = Alphanumeric.sample_iter(&mut rng).take(10).collect();

            let tx = create_memo_tx(&msg, &payer, blockhash);
            if tpu_client.send_transaction(&tx).await {
                txs.push(tx.signatures[0]);
            }
        }
        if enable_confirmation {
            txs_sent_deque.lock().await.push(txs);
        }
    }
}
