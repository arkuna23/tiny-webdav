use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
};

use crate::{DEFAULT_ADDR, DEFAULT_PORT};
use anyhow::anyhow;

#[derive(Debug, Clone)]
pub struct DavConfig {
    pub sock_addr: SocketAddr,
    pub dirs: Vec<DavDirConfig>,
}

#[derive(Debug, Clone)]
pub struct DavDirConfig {
    pub path: String,
    pub name: String,
}

fn get_dir_name(path: &str) -> String {
    let path = Path::new(&path);

    match path.file_name() {
        Some(path) => path.to_str().unwrap().to_owned(),
        None => "rootdir".to_owned(),
    }
}

/// parse dir string to path and name
fn parse_dir_str(string: &str) -> Option<(String, String)> {
    let mut iter = string.split('@');
    let path = iter.next()?.to_owned();
    let name = iter
        .next()
        .map(|r| r.to_owned())
        .unwrap_or_else(|| get_dir_name(&path));
    Some((path, name))
}

fn parse_dir_args(path_strs: Vec<String>) -> anyhow::Result<Vec<DavDirConfig>> {
    path_strs
        .into_iter()
        .map(|s| {
            let (path, name) =
                parse_dir_str(&s).ok_or_else(|| anyhow!("invalid dir string format"))?;
            Ok(DavDirConfig { path, name })
        })
        .collect::<anyhow::Result<_>>()
}

impl DavConfig {
    pub fn load_from_args(args: crate::Args) -> anyhow::Result<Self> {
        Ok(DavConfig {
            sock_addr: format!(
                "{}:{}",
                args.addr.unwrap_or(DEFAULT_ADDR.to_owned()),
                args.port.unwrap_or(DEFAULT_PORT)
            )
            .parse()?,
            dirs: parse_dir_args(args.dir.unwrap_or_else(|| vec!["./".to_owned()]))?,
        })
    }

    #[cfg(feature = "ini")]
    pub fn load(args: crate::Args, ini: ini::Ini) -> anyhow::Result<Self> {
        let global = ini
            .section(None::<String>)
            .ok_or_else(|| anyhow!("missing field global"))?;

        let sock_addr = {
            let addr = args
                .addr
                .or_else(|| global.get("addr").map(ToOwned::to_owned))
                .unwrap_or_else(|| DEFAULT_ADDR.to_owned());
            let port = {
                if let Some(port) = args.port {
                    port
                } else {
                    if let Some(port) = global.get("port") {
                        port.parse()?
                    } else {
                        DEFAULT_PORT
                    }
                }
            };
            format!("{}:{}", addr, port).parse()?
        };
        let dirs = {
            let mut dirs = parse_dir_args(args.dir.unwrap_or_default())?;
            for ele in ini.section_all(Some("Dir")) {
                let path = ele
                    .get("path")
                    .ok_or_else(|| anyhow!("missing field `path` in Dir section"))?;
                dirs.push(DavDirConfig {
                    path: path.to_owned(),
                    name: get_dir_name(path),
                });
            }
            if dirs.len() == 0 {
                dirs.push(DavDirConfig {
                    path: "./".to_owned(),
                    name: get_dir_name("./"),
                });
            }
            dirs
        };
        Ok(DavConfig { sock_addr, dirs })
    }
}
