use rusqlite::{params, Connection};
use std::sync::{Arc, Mutex};
use std::fs;
use tracing::{info, warn};
use siem::LogEvent;
use tokio::sync::mpsc;
use tokio::time::{self, Duration, Instant};

pub struct Storage {
    pub tx: mpsc::Sender<LogEvent>,
    pub warm_db: Arc<Mutex<Connection>>,
}

impl Storage {
    pub fn new() -> Self {
        fs::create_dir_all("./storage/cold").unwrap();

        // MPSC channel: Capacity 10000 to buffer bursts
        let (tx, rx) = mpsc::channel::<LogEvent>(10000);

        // Spawn background database writer
        tokio::spawn(database_writer_worker(rx));

        let warm_conn = Connection::open("warm_logs.db").unwrap();
        warm_conn.execute(
            "CREATE TABLE IF NOT EXISTS logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                severity TEXT NOT NULL,
                source_ip TEXT NOT NULL,
                facility TEXT NOT NULL,
                message TEXT NOT NULL
            );",
            [],
        ).unwrap();

        Storage {
            tx,
            warm_db: Arc::new(Mutex::new(warm_conn)),
        }
    }

    pub async fn send_log(&self, event: LogEvent) -> Result<(), mpsc::error::SendError<LogEvent>> {
        self.tx.send(event).await
    }
}

async fn database_writer_worker(mut rx: mpsc::Receiver<LogEvent>) {
    let mut conn = Connection::open("hot_logs.db").expect("Failed to open hot db");
    conn.busy_timeout(Duration::from_secs(5)).expect("Failed to set busy timeout");
    conn.pragma_update(None, "journal_mode", "WAL").expect("Failed to set WAL mode");
    
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT NOT NULL,
            severity TEXT NOT NULL,
            source_ip TEXT NOT NULL,
            facility TEXT NOT NULL,
            message TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_timestamp ON logs(timestamp);
        CREATE INDEX IF NOT EXISTS idx_source_ip ON logs(source_ip);",
    ).expect("Failed to initialize hot database schema");

    let mut batch = Vec::with_capacity(5000);
    let mut interval = time::interval(Duration::from_millis(500));
    let mut last_flush = Instant::now();

    loop {
        tokio::select! {
            Some(event) = rx.recv() => {
                batch.push(event);
                if batch.len() >= 5000 {
                    flush_batch(&mut conn, &mut batch);
                    last_flush = Instant::now();
                }
            }
            _ = interval.tick() => {
                if !batch.is_empty() && last_flush.elapsed() >= Duration::from_millis(500) {
                    flush_batch(&mut conn, &mut batch);
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
                event.timestamp.to_rfc3339(), 
                event.severity, 
                event.source_ip, 
                event.facility, 
                event.message
            ]).unwrap();
        }
    }
    tx.commit().unwrap();
    info!("Flushed batch of logs to Hot storage");
}

pub async fn run_janitor(storage: Arc<Storage>) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
    loop {
        interval.tick().await;
        info!("Running Janitor task...");

        // Threshold: older than 1 hour
        let threshold = (chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
        
        let storage = Arc::clone(&storage);
        
        tokio::task::spawn_blocking(move || {
            let hot_conn = Connection::open("hot_logs.db").expect("Failed to open hot db");
            hot_conn.busy_timeout(Duration::from_secs(5)).expect("Failed to set busy timeout");
            
            // Check if table exists
            {
                let _warm_conn = storage.warm_db.lock().expect("Failed to lock warm db");
                let table_exists: bool = hot_conn.query_row(
                    "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='logs'",
                    [],
                    |row| row.get(0),
                ).unwrap_or(0) > 0;

                if !table_exists {
                    warn!("Janitor: Logs table does not exist yet, skipping migration.");
                    return;
                }
            } 

            // Extract data to memory to avoid ATTACH lock contention
            let logs: Vec<(String, String, String, String, String)> = hot_conn.prepare(
                "SELECT timestamp, severity, source_ip, facility, message FROM logs WHERE timestamp < ?1"
            ).expect("Failed to prepare select")
            .query_map(params![threshold], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
            }).expect("Failed to query hot logs")
            .collect::<Result<_, _>>().expect("Failed to collect logs");

            if logs.is_empty() {
                return;
            }

            // Move data to Warm tier via dedicated connection
            {
                let mut warm_conn = storage.warm_db.lock().expect("Failed to lock warm db for migration");
                let tx = warm_conn.transaction().expect("Failed to start warm tx");
                for log in &logs {
                    tx.execute(
                        "INSERT INTO logs (timestamp, severity, source_ip, facility, message) VALUES (?1, ?2, ?3, ?4, ?5)",
                        params![log.0, log.1, log.2, log.3, log.4],
                    ).expect("Failed to insert into warm");
                }
                tx.commit().expect("Failed to commit warm tx");
                
                // Optimize warm db
                warm_conn.execute("PRAGMA optimize;", []).expect("Failed to optimize warm DB");
            }
            
            // Delete from Hot
            hot_conn.execute("DELETE FROM logs WHERE timestamp < ?1", params![threshold]).expect("Failed to delete from hot");
            
            info!("Janitor: Migrated {} logs older than {} to Warm tier", logs.len(), threshold);
        }).await.expect("Janitor task panicked");
    }
}
