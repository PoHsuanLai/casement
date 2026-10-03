//! The D-Bus connections casement holds (`casement::IpcState::handles`).

/// The live D-Bus connections. P0: the Debug1 server under `debug-inject`; P2-Z adds
/// `org.quire.Compositor1`.
#[derive(Debug, Default)]
pub struct IpcHandles {
    /// The private bus connection serving Debug1, once started.
    #[cfg(feature = "debug-inject")]
    pub debug: Option<zbus::Connection>,
}

impl IpcHandles {
    /// No connection yet.
    pub fn none() -> IpcHandles {
        IpcHandles::default()
    }
}
