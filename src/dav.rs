use std::{future, time::SystemTime};

use dav_server::fs::{DavDirEntry, DavFile, DavMetaData};

use crate::notimplemented_fut;

#[derive(Clone, Debug)]
pub struct FsMeta(u64);

impl Default for FsMeta {
    fn default() -> Self {
        Self(1)
    }
}

impl DavMetaData for FsMeta {
    fn len(&self) -> u64 {
        self.0
    }

    fn modified(&self) -> dav_server::fs::FsResult<std::time::SystemTime> {
        Ok(SystemTime::now())
    }

    fn is_dir(&self) -> bool {
        true
    }
}

#[derive(Debug)]
pub struct FsAsFile {
    len: u64,
}

impl Default for FsAsFile {
    fn default() -> Self {
        Self { len: 1 }
    }
}

#[allow(unused)]
impl DavFile for FsAsFile {
    fn metadata(&mut self) -> dav_server::fs::FsFuture<Box<dyn DavMetaData>> {
        Box::pin(future::ready(Ok(Box::new(FsMeta(self.len)) as _)))
    }

    fn write_buf(&mut self, buf: Box<dyn bytes::Buf + Send>) -> dav_server::fs::FsFuture<()> {
        notimplemented_fut!("write_buf")
    }

    fn write_bytes(&mut self, buf: bytes::Bytes) -> dav_server::fs::FsFuture<()> {
        notimplemented_fut!("write_bytes")
    }

    fn read_bytes(&mut self, count: usize) -> dav_server::fs::FsFuture<bytes::Bytes> {
        notimplemented_fut!("read_bytes")
    }

    fn seek(&mut self, pos: std::io::SeekFrom) -> dav_server::fs::FsFuture<u64> {
        notimplemented_fut!("seek")
    }

    fn flush(&mut self) -> dav_server::fs::FsFuture<()> {
        notimplemented_fut!("flush")
    }
}
pub struct FsAsDir {
    name: String,
    len: u64,
}

impl DavDirEntry for FsAsDir {
    fn name(&self) -> Vec<u8> {
        self.name.clone().into_bytes()
    }

    fn metadata(&self) -> dav_server::fs::FsFuture<Box<dyn dav_server::fs::DavMetaData>> {
        Box::pin(future::ready(Ok(Box::new(FsMeta(self.len)) as _)))
    }
}

impl FsAsDir {
    pub fn new(name: String) -> Self {
        Self { name, len: 1 }
    }
}
