use std::{fs::File, io::{Read, Seek, Write, BufReader}};
use fs2::FileExt;
use crate::error::Error;

pub(crate) struct SharedFileLock(BufReader<File>);
impl SharedFileLock {
    pub(crate) fn new(f: File) -> Result<Self, Error> {
        f.lock_shared()
            .map_err(|e| Error::LockError(e.to_string()))?;

        Ok(SharedFileLock(BufReader::new(f)))
    }
}
impl Drop for SharedFileLock {
    fn drop(&mut self) {
        let file = self.0.get_mut();
        file.unlock().expect("Failed to unlock file")
    }
}
impl Read for SharedFileLock {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buf)
    }
}
impl Seek for SharedFileLock {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.0.seek(pos)
    }
}

pub(crate) struct ExclusiveFileLock(File);
impl ExclusiveFileLock {
    pub(crate) fn new(f: File) -> Result<Self, Error> {
        f.lock_exclusive()
            .map_err(|e| Error::LockError(e.to_string()))?;

        Ok(Self(f))
    }
}
impl Drop for ExclusiveFileLock {
    fn drop(&mut self) {
        self.0.unlock().expect("Failed to unlock file")
    }
}
impl Read for ExclusiveFileLock {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buf)
    }
}
impl Write for ExclusiveFileLock {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }
}
impl Seek for ExclusiveFileLock {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.0.seek(pos)
    }
}
impl AsRef<File> for ExclusiveFileLock {
    fn as_ref(&self) -> &File {
        &self.0
    }
}
impl AsMut<File> for ExclusiveFileLock {
    fn as_mut(&mut self) -> &mut File {
        &mut self.0
    }
}
