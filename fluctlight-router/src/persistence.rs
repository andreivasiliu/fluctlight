use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use file_lock::{FileLock, FileOptions};
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

use crate::matrix_types::{Event, Id};

pub(crate) struct RoomPersistence {
    storage_path: PathBuf,
    state_pdu_file: GzEncoder<File>,
    other_pdu_file: GzEncoder<File>,
    file_lock: FileLock,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct PDUBlob<'a> {
    #[serde(borrow)]
    pub event_id: &'a Id<Event>,
    pub pdu_blob: &'a RawValue,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct OwnedPDUBlob {
    pub event_id: Box<Id<Event>>,
    pub pdu_blob: Box<RawValue>,
}

impl RoomPersistence {
    pub(crate) fn new(storage_path: impl Into<PathBuf>) -> Result<Self, std::io::Error> {
        let storage_path: PathBuf = storage_path.into();

        // FIXME: Better error reporting.
        let file_lock = lock_storage(&storage_path.join("lockfile"))?;

        // FIXME: Better error reporting.
        Ok(RoomPersistence {
            state_pdu_file: gzip_writer(&storage_path.join("state_pdus.json.gz"))?,
            other_pdu_file: gzip_writer(&storage_path.join("other_pdus.json.gz"))?,
            file_lock,
            storage_path,
        })
    }

    pub(crate) fn state_pdu_file(&mut self) -> &mut GzEncoder<File> {
        &mut self.state_pdu_file
    }

    pub(crate) fn read_state_pdu_file(&mut self) -> Result<Vec<u8>, std::io::Error> {
        // Opening it again is safe because we own the file-lock
        self.state_pdu_file.flush()?;

        let file = std::fs::File::open(self.storage_path.join("state_pdus.json.gz"))?;
        let mut file = GzDecoder::new(file);
        let mut contents = Vec::with_capacity(32 * 1024 * 1024);
        file.read_to_end(&mut contents).unwrap();

        Ok(contents)
    }
}

impl Drop for RoomPersistence {
    fn drop(&mut self) {
        // Attempt to flush before unlocking
        self.state_pdu_file.flush().ok();
        self.other_pdu_file.flush().ok();
        self.file_lock.unlock().ok();
    }
}

fn lock_storage(file_path: &Path) -> Result<FileLock, std::io::Error> {
    let is_blocking = false;

    let file_options = FileOptions::new().write(true).create(true).append(true);

    FileLock::lock(file_path, is_blocking, file_options)
}

fn gzip_writer(file_path: &Path) -> Result<GzEncoder<File>, std::io::Error> {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)?;

    Ok(GzEncoder::new(file, Compression::default()))
}
