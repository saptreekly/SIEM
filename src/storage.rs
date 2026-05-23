use rusqlite::{params, Connection};
use std::fs;
use tracing::info;
use siem::LogEvent;
use tokio::sync::mpsc;
use tokio::time::{self, Duration, Instant};

pub enum StorageMessage {
    Insert(LogEvent),
    Maintenance,
}

pub struct Storage {
    pub tx: mpsc::Sender<StorageMessage>,
}

impl Storage {
    pub fn new() -> Self {
        fs::create_dir_all("./storage/hot").unwrap();
        fs::create_dir_all("./storage/warm").unwrap();
        fs::create_dir_all("./storage/cold").unwrap();

        // MPSC channel: Capacity 10000 to buffer bursts
        let (tx, rx) = mpsc::channel::<StorageMessage>(10000);

        // Spawn background database actor
        tokio::spawn(database_actor(rx));

        Storage { tx }
    }
}

async fn database_actor(mut rx: mpsc::Receiver<StorageMessage>) {
    let mut hot_conn = Connection::open("storage/hot/hot_logs.db").expect("Failed to open hot db");
    hot_conn.busy_timeout(Duration::from_secs(5)).expect("Failed to set busy timeout");
    hot_conn.pragma_update(None, "journal_mode", "WAL").expect("Failed to set WAL mode");
    
    hot_conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp INTEGER NOT NULL,
            severity TEXT NOT NULL,
            source_ip TEXT NOT NULL,
            facility TEXT NOT NULL,
            message TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_timestamp ON logs(timestamp);
        CREATE INDEX IF NOT EXISTS idx_source_ip ON logs(source_ip);",
    ).expect("Failed to initialize hot database schema");

    let mut warm_conn = Connection::open("storage/warm/warm_logs.db").expect("Failed to open warm db");
    warm_conn.execute(
        "CREATE TABLE IF NOT EXISTS logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp INTEGER NOT NULL,
            severity TEXT NOT NULL,
            source_ip TEXT NOT NULL,
            facility TEXT NOT NULL,
            message TEXT NOT NULL
        );",
        [],
    ).expect("Failed to initialize warm database schema");

    let mut batch = Vec::with_capacity(5000);
    let mut interval = time::interval(Duration::from_millis(500));
    let mut last_flush = Instant::now();

    loop {
        tokio::select! {
            Some(msg) = rx.recv() => {
                match msg {
                    StorageMessage::Insert(event) => {
                        batch.push(event);
                        if batch.len() >= 5000 {
                            flush_batch(&mut hot_conn, &mut batch);
                            last_flush = Instant::now();
                        }
                    }
                    StorageMessage::Maintenance => {
                        // Janitor migration logic (In-memory buffer to avoid ATTACH lock contention)
                        let threshold = chrono::Utc::now().timestamp() - 3600;
                        
                        let logs: Vec<(i64, String, String, String, String)> = hot_conn.prepare(
                            "SELECT timestamp, severity, source_ip, facility, message FROM logs WHERE timestamp < ?1"
                        ).expect("Failed to prepare select")
                        .query_map(params![threshold], |row| {
                            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
                        }).expect("Failed to query hot logs")
                        .collect::<Result<_, _>>().expect("Failed to collect logs");

                        if !logs.is_empty() {
                            let tx = warm_conn.transaction().expect("Failed to start warm tx");
                            for log in &logs {
                                tx.execute(
                                    "INSERT INTO logs (timestamp, severity, source_ip, facility, message) VALUES (?1, ?2, ?3, ?4, ?5)",
                                    params![log.0, log.1, log.2, log.3, log.4],
                                ).expect("Failed to insert into warm");
                            }
                            tx.commit().expect("Failed to commit warm tx");
                            
                            hot_conn.execute("DELETE FROM logs WHERE timestamp < ?1", params![threshold]).expect("Failed to delete from hot");
                            info!("Janitor: Migrated {} logs to Warm tier", logs.len());
                            warm_conn.execute("PRAGMA optimize;", []).expect("Failed to optimize warm DB");
                        }
                    }
                }
            }
            _ = interval.tick() => {
                if !batch.is_empty() && last_flush.elapsed() >= Duration::from_millis(500) {
                    flush_batch(&mut hot_conn, &mut batch);
                    last_flush = Instant::now();
                }
            }
        }
    }
}

fn flush_batch(conn: &mut Connection, batch: &mut Vec<LogEvent>) {
    let tx = conn.transaction().unwrap();
    {
        let mut stmt = tx.prepare(
            "INSERT INTO logs (timestamp, severity, source_ip, facility, message) VALUES (?1, ?2, ?3, ?4, ?5)"
        ).unwrap();
        
        for event in batch.drain(..) {
            stmt.execute(params![
                event.timestamp, 
                event.severity.as_str(), 
                event.source_ip.as_str(), 
                event.facility.as_str(), 
                event.message.as_str()
            ]).unwrap();
        }
    }
    tx.commit().unwrap();
    info!("Flushed batch of logs to Hot storage");
}

pub async fn run_janitor(tx: mpsc::Sender<StorageMessage>) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
    loop {
        interval.tick().await;
        info!("Sending Maintenance signal...");
        let _ = tx.send(StorageMessage::Maintenance).await;
    }
}
