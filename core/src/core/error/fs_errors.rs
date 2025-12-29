#[derive(Debug)]
pub enum FsError {
    FileNotFound(String),
    NotAFile(String),
    NotADir(String),
    ReadError(String),
    WriteError(String),
    DeleteError(String),
}
