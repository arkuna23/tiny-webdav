#[macro_export]
macro_rules! notimplemented_fut {
    ($method:expr) => {{
        log::warn!("not implemented: {}", $method);
        Box::pin(std::future::ready(Err(
            dav_server::fs::FsError::NotImplemented,
        )))
    }};
}
