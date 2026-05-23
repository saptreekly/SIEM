use tokio::net::TcpStream;
use tokio::io::AsyncWriteExt;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stream = TcpStream::connect("127.0.0.1:8080").await?;
    let mut writer = stream;
    let log_line = "<34>1 2026-05-23T16:00:00Z localhost sshd[1234]: Failed password for root
";
    
    let iterations = 10_000;
    let start = Instant::now();
    
    for _ in 0..iterations {
        writer.write_all(log_line.as_bytes()).await?;
    }
    
    let duration = start.elapsed();
    let eps = iterations as f64 / duration.as_secs_f64();
    
    println!("Sent {} logs in {:?}. Throughput: {:.2} EPS", iterations, duration, eps);
    
    Ok(())
}
