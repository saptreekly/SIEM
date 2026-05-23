use rusqlite::{params, Connection};
use std::fs;
use std::thread;
use std::process::Command;
use tracing::{info};
use siem::LogEvent;
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::time::{Duration, Instant};

pub enum StorageMessage {
    Insert(LogEvent),
    Maintenance,
}

pub struct Storage {
    pub tx: Sender<StorageMessage>,
}

impl Storage {
    pub fn new() -> Self {
        fs::create_dir_all("./storage/hot").unwrap();
        fs::create_dir_all("./storage/warm").unwrap();
        fs::create_dir_all("./storage/cold").unwrap();

        // Fetch key from Zig agent
        let output = Command::new("./tools/key_agent")
            .output()
            .expect("Failed to execute key_agent");
        
        let key = String::from_utf8(output.stdout).expect("Failed to parse key");
        let key = key.trim().to_string();

        let (tx, rx) = unbounded::<StorageMessage>();

        thread::spawn(move || {
            database_actor_sync(rx, key);
        });

        Storage { tx }
    }
}

fn database_actor_sync(rx: Receiver<StorageMessage>, key: String) {
    let mut hot_conn = Connection::open("storage/hot/hot_logs.db").expect("Failed to open hot db");
    hot_conn.pragma_update(None, "key", &key).expect("Failed to set hot DB key");
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
    warm_conn.pragma_update(None, "key", &key).expect("Failed to set warm DB key");
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
    let mut last_flush = Instant::now();

    loop {
        // Block until message or timeout
        match rx.recv_timeout(Duration::from_millis(500)) {
            Ok(msg) => {
                match msg {
                    StorageMessage::Insert(event) => {
                        batch.push(event);
                        if batch.len() >= 5000 {
                            flush_batch(&mut hot_conn, &mut batch);
                            last_flush = Instant::now();
                        }
                    }
                    StorageMessage::Maintenance => {
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
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                if !batch.is_empty() && last_flush.elapsed() >= Duration::from_millis(500) {
                    flush_batch(&mut hot_conn, &mut batch);
                    last_flush = Instant::now();
                }
            }
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
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

pub fn run_janitor(tx: Sender<StorageMessage>) {
    loop {
        thread::sleep(Duration::from_secs(60));
        info!("Sending Maintenance signal...");
        let _ = tx.send(StorageMessage::Maintenance);
    }
}
