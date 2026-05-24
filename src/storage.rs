use crate::shm::ShmRingBuffer;
use bincode;
use crossbeam_channel::{unbounded, Receiver, Sender};
use siem::LogEvent;
use std::collections::BTreeMap;
use std::fs;
use std::io::{BufWriter, Write};
use std::thread;
use std::time::Duration;
use tracing::info;

pub enum StorageMessage {
    Insert(LogEvent),
    Maintenance,
    Shutdown,
}

pub struct Storage {
    pub tx: Sender<StorageMessage>,
}

impl Storage {
    pub fn new(shm: Option<ShmRingBuffer>) -> Self {
        fs::create_dir_all("./storage/hot").unwrap();
        fs::create_dir_all("./storage/warm").unwrap();
        fs::create_dir_all("./storage/cold").unwrap();

        let (tx, rx) = unbounded::<StorageMessage>();
        std::thread::spawn(move || {
            database_actor_sync(rx, shm);
        });
        Storage { tx }
    }
}

struct MemTable {
    data: BTreeMap<i64, LogEvent>,
}

fn database_actor_sync(rx: Receiver<StorageMessage>, mut shm: Option<ShmRingBuffer>) {
    let mut memtable = MemTable {
        data: BTreeMap::new(),
    };
    let wal_file = fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open("storage/hot/wal.log")
        .expect("Failed to open WAL");

    let mut wal_writer = BufWriter::new(wal_file);

    loop {
        match rx.recv_timeout(Duration::from_millis(500)) {
            Ok(StorageMessage::Insert(event)) => {
                // Write to WAL first
                let encoded = bincode::serialize(&event).expect("Failed to serialize");
                wal_writer
                    .write_all(&encoded)
                    .expect("Failed to write to WAL");
                wal_writer
                    .write_all(
                        b"
",
                    )
                    .expect("Failed to write newline to WAL");
                wal_writer.flush().expect("Failed to flush WAL");

                // Write to SHM
                if let Some(ref mut shm_buf) = shm {
                    shm_buf.write_event(&event);
                }

                // Update MemTable
                memtable.data.insert(event.timestamp, event);

                if memtable.data.len() >= 5000 {
                    info!("MemTable full, flushing to SSTable (STUB)");
                    memtable.data.clear();
                }
            }
            Ok(StorageMessage::Maintenance) => {
                info!("Running LSM compaction (STUB)...");
            }
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => continue,
            Ok(StorageMessage::Shutdown) => {
                info!("Storage actor shutting down...");
                break;
            }
            Err(_) => break,
        }
    }
}

pub fn run_janitor(tx: Sender<StorageMessage>) {
    loop {
        thread::sleep(Duration::from_secs(60));
        info!("Sending Maintenance signal...");
        let _ = tx.send(StorageMessage::Maintenance);
    }
}
