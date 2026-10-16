//! Whether the program behind a connection still runs the image it was vouched for at accept.
//! The connection holds the process by a descriptor of it (a process cannot be replaced while
//! it is held), so what `/proc/<pid>/exe` says belongs to the same process: if it names another
//! image now, the program executed something else after it connected, and what it was allowed
//! as the old image says nothing about the new one.

use super::standing::image_name;
use casement_core::privilege::ClientStanding;
use casement_ipc::dbus::RealProc;
use casement_ipc::dbus::peer::{Exe, Pinned, Prober};
use casement_wayland::consent::ImageWatch;

/// A held process and the image it had at accept.
#[derive(Debug)]
pub(super) struct PinnedImage {
    pinned: Pinned,
    prober: Prober<RealProc>,
    accepted: Exe,
}

impl PinnedImage {
    /// The watch over `pinned`, whose image was `accepted`.
    pub(super) fn new(pinned: Pinned, prober: Prober<RealProc>, accepted: Exe) -> PinnedImage {
        PinnedImage {
            pinned,
            prober,
            accepted,
        }
    }
}

impl ImageWatch for PinnedImage {
    fn moved(&self) -> Option<ClientStanding> {
        // A process that left, or a file that cannot be read now, is the model's to refuse:
        // only an image that is readable and different counts as moved.
        let facts = self.prober.probe(&self.pinned, None).ok()?;
        (facts.exe != self.accepted).then(|| ClientStanding::Unverified {
            exe_name: image_name(&facts.exe),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_ipc::dbus::peer::{ExePath, FileTrust};
    use std::os::unix::net::UnixStream;

    #[test]
    fn a_process_that_runs_another_image_than_it_was_vouched_for_is_unverified() {
        let (ours, _theirs) = UnixStream::pair().expect("a socket pair");
        let pinned = Pinned::of_stream(&ours).expect("this process is held");
        let prober = Prober::system();
        let actual = prober
            .probe(&pinned, None)
            .expect("this process is read")
            .exe;
        let same = PinnedImage::new(
            Pinned::of_stream(&ours).expect("held"),
            prober.clone(),
            actual,
        );
        assert!(same.moved().is_none(), "the image it had");
        let elsewhere = Exe::Read {
            path: ExePath::new("/usr/bin/somewhere-else").expect("a path"),
            trust: FileTrust::RootOnly,
        };
        let moved = PinnedImage::new(pinned, prober, elsewhere);
        assert!(
            matches!(moved.moved(), Some(ClientStanding::Unverified { .. })),
            "another image than the one vouched for"
        );
    }
}
