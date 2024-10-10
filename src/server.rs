use std::{collections::HashMap, convert::Infallible, net::SocketAddr, sync::Arc};

use dav_server::{
    davpath::DavPath,
    fakels::FakeLs,
    fs::{DavDirEntry, FsError, FsResult, GuardedFileSystem},
    localfs::LocalFs,
    DavHandler,
};
use hyper::server::conn::http1;
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

use crate::{
    config::{DavConfig, DavDirConfig},
    dav::{FsDirEntry, FsFile, FsMeta},
};

type FsMap = Arc<HashMap<String, Box<LocalFs>>>;
pub struct DavServer {
    handler: DavHandler<Cred>,
    addr: SocketAddr,
}

impl DavServer {
    pub fn new(config: DavConfig) -> Self {
        Self {
            handler: DavHandler::builder()
                .filesystem(Box::new(MultiFs::new(config.dirs)))
                .locksystem(FakeLs::new())
                .build_handler(),
            addr: config.sock_addr,
        }
    }

    #[inline(always)]
    pub async fn run(self) -> anyhow::Result<()> {
        Self::server_loop(self.handler, self.addr).await
    }

    async fn server_loop(handler: DavHandler<Cred>, addr: SocketAddr) -> anyhow::Result<()> {
        let listener = TcpListener::bind(addr).await?;
        loop {
            let (stream, addr) = listener.accept().await?;
            let handler = handler.clone();

            let io = TokioIo::new(stream);

            tokio::spawn(async move {
                log::debug!("serve connection: {addr}");
                if let Err(err) = http1::Builder::new()
                    .serve_connection(
                        io,
                        hyper::service::service_fn({
                            move |req| {
                                let handler = handler.clone();
                                async move {
                                    Ok::<_, Infallible>(handler.handle_guarded(req, Cred).await)
                                }
                            }
                        }),
                    )
                    .await
                {
                    log::error!("Failed serving: {err:?}");
                }
            });
        }
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
        log::debug!("req path: {coms:?}");
        // if get root path(because root path showing all fs, can't be parsed)
        let name = coms.remove(0);
        if name.is_empty() {
            return Ok(None);
        }
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

macro_rules! fs_action {
    ($self:expr, $fs_func:ident, $root_path_action:expr, $path:expr, $($args:tt)*) => {
        Box::pin(async move {
            log::debug!("fs action: {}", stringify!($fs_func));
            if $self.fs_map.len() > 1 {
                if let Some(DavPathParts { fs_name, path }) = MultiFs::parse_dav_path($path)? {
                    $self.get_fs(&fs_name)?.$fs_func(&path, $($args)*).await
                } else {
                    $root_path_action
                }
            } else {
                $self.get_first_fs().$fs_func($path, $($args)*).await
            }
        })
    };
}

impl GuardedFileSystem<Cred> for MultiFs {
    fn open<'a>(
        &'a self,
        path: &'a DavPath,
        options: dav_server::fs::OpenOptions,
        _credentials: &'a Cred,
    ) -> dav_server::fs::FsFuture<Box<dyn dav_server::fs::DavFile>> {
        fs_action!(
            self,
            open,
            Ok(Box::new(FsFile::default()) as _),
            &path,
            options,
            &()
        )
    }

    fn read_dir<'a>(
        &'a self,
        path: &'a DavPath,
        meta: dav_server::fs::ReadDirMeta,
        _credentials: &'a Cred,
    ) -> dav_server::fs::FsFuture<dav_server::fs::FsStream<Box<dyn dav_server::fs::DavDirEntry>>>
    {
        fs_action!(
            self,
            read_dir,
            {
                let entries =
                    self.fs_map
                        .keys()
                        .map(|k| {
                            FsResult::Ok(
                                Box::new(FsDirEntry::new(k.to_owned())) as Box<dyn DavDirEntry>
                            )
                        })
                        .collect::<Vec<_>>();
                Ok(Box::pin(futures_util::stream::iter(entries)) as _)
            },
            &path,
            meta,
            &()
        )
    }

    fn metadata<'a>(
        &'a self,
        path: &'a dav_server::davpath::DavPath,
        _credentials: &'a Cred,
    ) -> dav_server::fs::FsFuture<Box<dyn dav_server::fs::DavMetaData>> {
        fs_action!(
            self,
            metadata,
            Ok(Box::new(FsMeta::default()) as _),
            &path,
            &()
        )
    }

    fn symlink_metadata<'a>(
        &'a self,
        path: &'a DavPath,
        _credentials: &'a Cred,
    ) -> dav_server::fs::FsFuture<Box<dyn dav_server::fs::DavMetaData>> {
        fs_action!(
            self,
            symlink_metadata,
            Ok(Box::new(FsMeta::default()) as _),
            &path,
            &()
        )
    }

    fn create_dir<'a>(
        &'a self,
        path: &'a DavPath,
        _credentials: &'a Cred,
    ) -> dav_server::fs::FsFuture<()> {
        fs_action!(self, create_dir, Err(FsError::Forbidden), &path, &())
    }

    fn remove_dir<'a>(
        &'a self,
        path: &'a DavPath,
        _credentials: &'a Cred,
    ) -> dav_server::fs::FsFuture<()> {
        fs_action!(self, remove_dir, Err(FsError::Forbidden), &path, &())
    }

    fn remove_file<'a>(
        &'a self,
        path: &'a DavPath,
        _credentials: &'a Cred,
    ) -> dav_server::fs::FsFuture<()> {
        fs_action!(self, remove_file, Err(FsError::Forbidden), &path, &())
    }

    fn rename<'a>(
        &'a self,
        from: &'a DavPath,
        to: &'a DavPath,
        _credentials: &'a Cred,
    ) -> dav_server::fs::FsFuture<()> {
        fs_action!(self, rename, Err(FsError::Forbidden), &from, &to, &())
    }

    fn copy<'a>(
        &'a self,
        from: &'a DavPath,
        to: &'a DavPath,
        _credentials: &'a Cred,
    ) -> dav_server::fs::FsFuture<()> {
        fs_action!(
            self,
            copy,
            {
                log::warn!("copy cross fs not implemented");
                Err(FsError::NotImplemented)
            },
            &from,
            &to,
            &()
        )
    }
}
