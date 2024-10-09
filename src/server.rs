use std::{collections::HashMap, pin::Pin, sync::Arc};

use dav_server::{
    davpath::DavPath,
    fakels::FakeLs,
    fs::{DavDirEntry, FsError, FsResult, GuardedFileSystem},
    localfs::LocalFs,
    DavHandler,
};
use futures_util::Stream;

use crate::{
    config::{DavConfig, DavDirConfig},
    dav::FsDirEntry,
};

type FsMap = Arc<HashMap<String, Box<LocalFs>>>;
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

struct DavPathParts {
    fs_name: String,
    path: DavPath,
}

impl MultiFs {
    pub fn new(dirs: Vec<DavDirConfig>) -> Self {
        let fs_map = Arc::new(
            dirs.into_iter()
                .map(|r| (r.name, LocalFs::new(r.path, false, false, false)))
                .collect::<HashMap<_, _>>(),
        );
        Self { fs_map }
    }

    fn parse_dav_path(path: &DavPath) -> dav_server::fs::FsResult<Option<DavPathParts>> {
        let origin = path.as_url_string();
        let mut coms: Vec<_> = origin.trim_start_matches('/').split('/').collect();
        if coms.is_empty() {
            return Ok(None);
        }

        let name = coms.remove(0);
        let dav_path = {
            let mut dav_path = DavPath::new(&format!("/{}", coms.join("/"))).map_err(|e| {
                log::error!("{:?}", e);
                FsError::GeneralFailure
            })?;

            dav_path.set_prefix(path.prefix()).map_err(|e| {
                log::error!("{:?}", e);
                FsError::GeneralFailure
            })?;
            dav_path
        };

        Ok(Some(DavPathParts {
            fs_name: name.to_owned(),
            path: dav_path,
        }))
    }

    /// get fs and path in the fs
    #[inline(always)]
    fn get_fs<'a>(&self, name: &str) -> Result<&Box<LocalFs>, FsError> {
        self.fs_map.get(name).ok_or(FsError::NotFound)
    }

    #[inline(always)]
    fn get_first_fs(&self) -> &Box<LocalFs> {
        self.fs_map.iter().next().unwrap().1
    }
}

impl GuardedFileSystem<Cred> for MultiFs {
    fn open<'a>(
        &'a self,
        path: &'a DavPath,
        options: dav_server::fs::OpenOptions,
        _credentials: &'a Cred,
    ) -> dav_server::fs::FsFuture<Box<dyn dav_server::fs::DavFile>> {
        Box::pin(async move {
            if self.fs_map.len() > 1 {
                let DavPathParts { fs_name, path } =
                    Self::parse_dav_path(&path)?.ok_or(FsError::NotFound)?;
                let fs = self.get_fs(&fs_name)?;
                fs.open(&path, options, &()).await
            } else {
                self.get_first_fs().open(&path, options, &()).await
            }
        })
    }

    fn read_dir<'a>(
        &'a self,
        path: &'a DavPath,
        meta: dav_server::fs::ReadDirMeta,
        credentials: &'a Cred,
    ) -> dav_server::fs::FsFuture<dav_server::fs::FsStream<Box<dyn dav_server::fs::DavDirEntry>>>
    {
        Box::pin(async move {
            if self.fs_map.len() > 1 {
                if let Some(DavPathParts { fs_name, path }) = Self::parse_dav_path(&path)? {
                    self.get_fs(&fs_name)?.read_dir(&path, meta, &()).await
                } else {
                    let entries =
                        self.fs_map.keys().map(|k| {
                            FsResult::Ok(
                                Box::new(FsDirEntry::new(k.to_owned())) as Box<dyn DavDirEntry>
                            )
                        }).collect::<Vec<_>>();
                    Ok(Box::pin(futures_util::stream::iter()) as _)
                }
            } else {
                self.get_first_fs().read_dir(&path, meta, &()).await
            }
        })
    }

    fn metadata<'a>(
        &'a self,
        path: &'a dav_server::davpath::DavPath,
        credentials: &'a Cred,
    ) -> dav_server::fs::FsFuture<Box<dyn dav_server::fs::DavMetaData>> {
        todo!()
    }
}
