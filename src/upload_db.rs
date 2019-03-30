use std::fs::{self, File, DirEntry};
use std::path::{PathBuf};
use std::io;

pub struct FileEntry {
    file_name: String,
    len: u64
}

fn db_path() -> PathBuf {
    return PathBuf::from("/var/lib/upload-stick/uploaded");
}

fn entry_path(entry: &FileEntry) -> PathBuf {
    return db_path().join(format!("{}_{}", &entry.file_name, entry.len));
}

fn ensure_db_exists() -> io::Result<()> {
    return fs::create_dir_all(&db_path());
}

pub fn from_dir_entry(dir_entry: &DirEntry) -> io::Result<FileEntry> {
    return Ok(FileEntry {
        file_name: dir_entry.file_name().to_string_lossy().to_string(),
        len: dir_entry.metadata()?.len()
    });
}

pub fn is_uploaded(entry: &FileEntry) -> io::Result<bool> {
    ensure_db_exists()?;
    return match fs::metadata(entry_path(entry)) {
        Ok(metadata) => Ok(metadata.is_file()),
        Err(ref err) if err.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(err)
    };
}

pub fn set_uploaded(entry: &FileEntry) -> io::Result<()> {
    ensure_db_exists()?;
    File::create(entry_path(entry))?;
    return Ok(());
}
