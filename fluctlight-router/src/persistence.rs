use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

use file_lock::{FileLock, FileOptions};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

use crate::matrix_types::{Event, Id, ServerName};

pub(crate) struct RoomPersistence {
    pub state_pdu_file: PDUFile,
    pub other_pdu_file: PDUFile,
    // Planned:
    // index: BlockVec<PDULocation> (by IntStr)
    // Maybe BlockMap (that auto-grows with IntStr ID)

    // File: append-only
    // <event_id:44> <file:4> <offset:4> <length:4> =======\n

    // Figure out:
    // Simple storage files
    // For now just store everything
}

// struct PDULocation {
//     file: usize,
//     offset: usize,
//     length: usize,
// }

pub(crate) struct PDUFile {
    // gz_file: Option<GzEncoder<File>>,
    file: File,
    file_path: PathBuf,
    file_lock: FileLock,
}

impl PDUFile {
    fn new(file_path: PathBuf) -> Result<Self, std::io::Error> {
        let file_lock;
        // let gz_file;

        if !file_path.exists() {
            std::fs::write(&file_path, b"")?;
        }
        file_lock = lock_storage(&file_path)?;
        // gz_file = gzip_writer(&file_path)?;
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&file_path)?;

        Ok(PDUFile {
            // gz_file: Some(gz_file),
            file,
            file_path,
            file_lock,
        })
    }

    // pub(crate) fn read_contents(&mut self) -> Result<Vec<u8>, std::io::Error> {
    //     let file_encoder = self
    //         .gz_file
    //         .take()
    //         .expect("PDUFile should always have an opened gz_file");
    //     let mut file = file_encoder.finish()?;

    //     file.seek(SeekFrom::Start(0))?;

    //     eprintln!("Loading {}", self.file_path.display());
    //     let mut file_decoder = MultiGzDecoder::new(file);
    //     let mut contents = Vec::new();
    //     // FIXME: Figure out capacity (e.g. guesstimate based on compressed size)
    //     file_decoder.read_to_end(&mut contents).unwrap();

    //     let mut file = file_decoder.into_inner();
    //     file.seek(SeekFrom::End(0))?;

    //     let file_encoder = GzEncoder::new(file, Compression::default());
    //     self.gz_file = Some(file_encoder);

    //     Ok(contents)
    // }

    pub(crate) fn read_contents(&mut self) -> Result<Vec<u8>, std::io::Error> {
        self.file.seek(SeekFrom::Start(0))?;

        eprintln!("Loading {}", self.file_path.display());
        let mut contents = Vec::new();
        // FIXME: Figure out capacity (e.g. guesstimate based on compressed size)
        self.file.read_to_end(&mut contents).unwrap();

        self.file.seek(SeekFrom::End(0))?;

        Ok(contents)
    }

    pub(crate) fn read_pdus(&mut self, mut f: impl FnMut(PDUBlob)) {
        let pdu_file_contents = self.read_contents().unwrap();

        let json_stream = serde_json::Deserializer::from_slice(&pdu_file_contents);

        for pdu_blob in json_stream.into_iter::<PDUBlob>() {
            f(pdu_blob.unwrap())
        }
    }

    pub(crate) fn write_pdu(
        &mut self,
        event_id: &Id<Event>,
        origin: Option<&Id<ServerName>>,
        pdu_blob: &RawValue,
    ) {
        let pdu_blob = PDUBlob {
            event_id,
            origin,
            pdu_blob,
        };
        // FIXME: error
        // let gz_file = self
        //     .gz_file
        //     .as_mut()
        //     .expect("PDUFile should always have an opened gz_file");
        let file = &mut self.file;
        serde_json::to_writer(&mut *file, &pdu_blob)
            .expect("Could not write to persistent room storage");
        file
            .write(b"\n")
            .expect("Could not write to persistent room storage");
        file
            .flush()
            .expect("Could not flush room persistence store");
    }
}

impl Drop for PDUFile {
    fn drop(&mut self) {
        // if let Some(mut gz_file) = self.gz_file.take() {
        //     // Attempt to flush before unlocking
        //     // FIXME: log issues
        //     gz_file.flush().ok();
        // }
        self.file.flush().ok();
        self.file_lock.unlock().ok();
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct PDUBlob<'a> {
    #[serde(borrow)]
    pub event_id: &'a Id<Event>,
    pub origin: Option<&'a Id<ServerName>>,
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
        if !storage_path.exists() {
            std::fs::create_dir(&storage_path)?;
        }

        // FIXME: Better error reporting.
        Ok(RoomPersistence {
            state_pdu_file: PDUFile::new(storage_path.join("state_pdus.json").to_owned())?,
            other_pdu_file: PDUFile::new(storage_path.join("other_pdus.json").to_owned())?,
        })
    }
}

fn lock_storage(file_path: &Path) -> Result<FileLock, std::io::Error> {
    let is_blocking = false;

    let file_options = FileOptions::new().write(true).create(true).append(true);

    FileLock::lock(file_path, is_blocking, file_options)
}

// fn gzip_writer(file_path: &Path) -> Result<GzEncoder<File>, std::io::Error> {
//     let file = OpenOptions::new()
//         .create(true)
//         .append(true)
//         .read(true)
//         .open(file_path)?;

//     Ok(GzEncoder::new(file, Compression::default()))
// }
