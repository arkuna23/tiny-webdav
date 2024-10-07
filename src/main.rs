use clap::Parser;
use config::DavConfig;

mod config;
mod server;

const DEFAULT_PORT: u16 = 8080;
const DEFAULT_ADDR: &str = "127.0.0.1";

/// simple webdav server with multi dirs, no gui
#[derive(Debug, clap::Parser)]
#[command(version, about)]
pub struct Args {
    /// webdav server port [default: 8080]
    #[arg(short, long, value_name = "PORT")]
    pub port: Option<u16>,
    /// webdav server address [default: 127.0.0.1]
    #[arg(short, long, value_name = "ADDR")]
    pub addr: Option<String>,
    /// webdav server dirs, format: /path/to/dir@name, name is optional
    #[arg(short, long, value_name = "PATH")]
    pub dir: Option<Vec<String>>,

    #[cfg(feature = "ini")]
    /// config file path
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    #[cfg(feature = "ini")]
    let global_conf = {
        if let Some(file) = args.config.clone() {
            DavConfig::load(args, ini::Ini::load_from_file(file)?)?;
        } else {
            DavConfig::load_from_args(args)?;
        }
    };
    #[cfg(not(feature = "ini"))]
    let global_conf = DavConfig::load_from_args(args)?;

    println!("{:?}", global_conf);

    Ok(())
}