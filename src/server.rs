use std::collections::HashMap;

use dav_server::{
    davpath::DavPath,
    fakels::FakeLs,
    fs::{FsError, GuardedFileSystem},
    localfs::LocalFs,
    DavHandler,
};

use crate::config::{DavConfig, DavDirConfig};

type FsMap = HashMap<String, Box<LocalFs>>;
pub struct DavServer {
    handler: DavHandler<Cred>,
}

impl DavServer {
    pub fn new(config: DavConfig) -> Self {
        Self {
            handler: DavHandler::builder()
                .filesystem(Box::new(MultiFs::new(config.dirs)))
                .locksystem(FakeLs::new())
                .build_handler(),
        }
    }

    #[inline(always)]
    pub fn run(self) -> tokio::task::JoinHandle<anyhow::Result<()>> {
        tokio::spawn(Self::server_loop(self.handler))
    }

    async fn server_loop(handler: DavHandler<Cred>) -> anyhow::Result<()> {
        loop {}
    }
}

#[derive(Clone)]
pub struct MultiFs {
    fs_map: FsMap,
}

#[derive(Clone)]
pub struct Cred;

impl MultiFs {
    pub fn new(dirs: Vec<DavDirConfig>) -> Self {
        let fs_map = dirs
            .into_iter()
            .map(|r| (r.name, LocalFs::new(r.path, false, false, false)))
            .collect::<HashMap<_, _>>();
        Self { fs_map }
    }

    /// get fs and path in the fs
    fn get_fs<'a>(&self, path: &'a DavPath) -> Result<(&Box<LocalFs>, DavPath), FsError> {
        if self.fs_map.len() == 1 {
            let fs = self.fs_map.iter().next().unwrap().1;
            Ok((&fs, path.clone()))
        } else {
            let origin = path.as_url_string();
            let mut coms: Vec<_> = origin.trim_start_matches('/').split('/').collect();

            let name = coms.remove(0);
            let mut dav_path = DavPath::new(&format!("/{}", coms.join("/"))).map_err(|e| {
                log::error!("{:?}", e);
                FsError::GeneralFailure
            })?;
            dav_path.set_prefix(path.prefix());
            self.fs_map
                .get(name)
                .ok_or(FsError::NotFound)
                .map(|r| (r, dav_path))
        }
    }
}

impl GuardedFileSystem<Cred> for MultiFs {
    fn open<'a>(
        &'a self,
        path: &'a DavPath,
        options: dav_server::fs::OpenOptions,
        credentials: &'a Cred,
    ) -> dav_server::fs::FsFuture<Box<dyn dav_server::fs::DavFile>> {
        let (fs, path) = self.get_fs(path).unwrap();
        fs.open(&path, options, &())
    }

    fn read_dir<'a>(
        &'a self,
        path: &'a DavPath,
        meta: dav_server::fs::ReadDirMeta,
        credentials: &'a Cred,
    ) -> dav_server::fs::FsFuture<dav_server::fs::FsStream<Box<dyn dav_server::fs::DavDirEntry>>>
    {
        todo!()
    }

    fn metadata<'a>(
        &'a self,
        path: &'a dav_server::davpath::DavPath,
        credentials: &'a Cred,
    ) -> dav_server::fs::FsFuture<Box<dyn dav_server::fs::DavMetaData>> {
        todo!()
    }
}
