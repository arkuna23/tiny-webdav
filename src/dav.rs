use std::{borrow::Cow, time::SystemTime};

use dav_server::fs::{DavDirEntry, DavMetaData};

pub struct FsDirEntry {
    name: String,
    len: u64,
}

impl DavDirEntry for FsDirEntry {
    fn name(&self) -> Vec<u8> {
        self.name.clone().into_bytes()
    }

    fn metadata(&self) -> dav_server::fs::FsFuture<Box<dyn dav_server::fs::DavMetaData>> {
        #[derive(Clone, Debug)]
        struct Metadata(u64);

        impl DavMetaData for Metadata {
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

        Box::pin(async move { Ok(Box::new(Metadata(self.len)) as _) })
    }
}

impl FsDirEntry {
    pub fn new(name: String) -> Self {
        Self { name, len: 1 }
    }
}
