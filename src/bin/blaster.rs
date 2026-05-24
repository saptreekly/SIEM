use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::net::TcpStream;
use tokio::sync::{oneshot, Barrier};

const CONCURRENT_WORKERS: u64 = 10;
const LOGS_PER_WORKER: u64 = 5_000;
const MOCK_LOG: &str = "<34>1 2026-05-23T16:00:00Z 192.168.1.50 sshd[28412]: Failed password for invalid user admin from 203.0.113.5 port 51234 ssh2
";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let counter = Arc::new(AtomicU64::new(0));
    let barrier = Arc::new(Barrier::new(CONCURRENT_WORKERS as usize + 1));
    let start_time = Instant::now();
    let (tx, mut rx) = oneshot::channel();

    // Telemetry task
    let telemetry_counter = Arc::clone(&counter);
    tokio::spawn(async move {
        let mut last_count = 0;
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let current_count = telemetry_counter.load(Ordering::Relaxed);
                    let eps = current_count - last_count;
                    println!("Current Throughput: {} EPS", eps);
                    last_count = current_count;
                }
                _ = &mut rx => {
                    break;
                }
            }
        }
    });

    // Spawn workers
    for _ in 0..CONCURRENT_WORKERS {
        let counter = Arc::clone(&counter);
        let barrier = Arc::clone(&barrier);
        tokio::spawn(async move {
            let port = std::fs::read_to_string("port.info").unwrap().trim().to_string();
            let addr = format!("127.0.0.1:{}", port);
            let mut backoff = Duration::from_millis(100);
            let stream = loop {
                match TcpStream::connect(&addr).await {
                    Ok(s) => break s,
                    Err(_) => {
                        tokio::time::sleep(backoff).await;
                        backoff = std::cmp::min(backoff * 2, Duration::from_secs(5));
                    }
                }
            };
            let mut writer = BufWriter::new(stream);

            barrier.wait().await; // Synchronize start

            for _ in 0..LOGS_PER_WORKER {
                if writer.write_all(MOCK_LOG.as_bytes()).await.is_err() {
                    break;
                }
                counter.fetch_add(1, Ordering::Relaxed);
            }
            let _ = writer.flush().await;
            barrier.wait().await; // Signal completion
        });
    }

    barrier.wait().await; // Wait for all workers to start
    barrier.wait().await; // Wait for all workers to finish
    let _ = tx.send(()); // Signal telemetry task to stop

    let elapsed = start_time.elapsed();
    let total_logs = counter.load(Ordering::Relaxed);
    let avg_eps = total_logs as f64 / elapsed.as_secs_f64();

    println!(
        "
--- Blaster Performance Report ---"
    );
    println!("Total Connections Opened: {}", CONCURRENT_WORKERS);
    println!("Total Logs Sent:         {}", total_logs);
    println!(
        "Total Time Elapsed:      {:.2} seconds",
        elapsed.as_secs_f64()
    );
    println!("Average Throughput:      {:.2} EPS", avg_eps);
    println!("----------------------------------");

    Ok(())
}
