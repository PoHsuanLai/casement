//! The `toplevel` and `layer` probes: map a window or a layer surface and print what the
//! compositor says about it. Self-contained wayland-client (a binary cannot add `Dispatch`
//! impls to the library's client state): its own state, its own bounded waits.

use super::Failure;
use casement_core::geometry::PhysicalSize;
use casement_testclient::Pattern;
use rustix::event::{PollFd, PollFlags, Timespec};
use rustix::fs::{MemfdFlags, ftruncate, memfd_create};
use serde_json::json;
use std::fs::File;
use std::ops::RangeInclusive;
use std::os::fd::AsFd;
use std::os::unix::fs::FileExt;
use wayland_client::globals::{GlobalList, GlobalListContents, registry_queue_init};
use wayland_client::protocol::wl_buffer::WlBuffer;
use wayland_client::protocol::wl_compositor::WlCompositor;
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_client::protocol::wl_shm::{Format, WlShm};
use wayland_client::protocol::wl_shm_pool::WlShmPool;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::protocol::{wl_buffer, wl_shm, wl_shm_pool, wl_surface};
use wayland_client::{Connection, Dispatch, EventQueue, Proxy, QueueHandle};
use wayland_protocols::xdg::shell::client::xdg_surface::{self, XdgSurface};
use wayland_protocols::xdg::shell::client::xdg_toplevel::{self, XdgToplevel};
use wayland_protocols::xdg::shell::client::xdg_wm_base::{self, XdgWmBase};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::{
    self, Layer, ZwlrLayerShellV1,
};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::{
    self, Anchor, ZwlrLayerSurfaceV1,
};

/// Rounds of two seconds a probe waits for the compositor before it gives up.
const ROUNDS: u32 = 10;
/// The size asked for when the compositor leaves it to the client.
const DEFAULT_SIZE: (u32, u32) = (200, 150);

#[derive(Debug, Default)]
struct State {
    /// The last size a toplevel or layer configure carried (0 means "your choice").
    configured: Option<(u32, u32)>,
    /// An `xdg_surface` or layer-surface configure has been acknowledged: safe to attach.
    acked: bool,
    closed: bool,
    /// The states the last toplevel configure carried.
    states: Vec<xdg_toplevel::State>,
}

/// The pieces both probes bind.
struct Bound {
    queue: EventQueue<State>,
    qh: QueueHandle<State>,
    globals: GlobalList,
    compositor: WlCompositor,
    shm: WlShm,
    state: State,
}

fn bind() -> Result<(Connection, Bound), Failure> {
    let conn = Connection::connect_to_env().map_err(|e| Failure::Probe(format!("connect: {e}")))?;
    let (globals, queue) = registry_queue_init::<State>(&conn)
        .map_err(|e| Failure::Probe(format!("registry: {e}")))?;
    let qh = queue.handle();
    let bound = Bound {
        queue,
        qh: qh.clone(),
        compositor: bind_in(&globals, &qh, 1..=4)?,
        shm: bind_in(&globals, &qh, 1..=1)?,
        globals,
        state: State::default(),
    };
    Ok((conn, bound))
}

/// Binds global `I` at a version in `versions`, or says it is not advertised.
fn bind_in<I>(
    globals: &GlobalList,
    qh: &QueueHandle<State>,
    versions: RangeInclusive<u32>,
) -> Result<I, Failure>
where
    I: Proxy + 'static,
    State: Dispatch<I, ()>,
{
    globals.bind(qh, versions, ()).map_err(|_| {
        Failure::Probe(format!(
            "the compositor does not advertise {}",
            I::interface().name
        ))
    })
}

/// Dispatches until `done` holds on the state, at most `ROUNDS` waits of two seconds each.
fn wait_for(bound: &mut Bound, what: &str, done: impl Fn(&State) -> bool) -> Result<(), Failure> {
    for _ in 0..ROUNDS {
        bound
            .queue
            .dispatch_pending(&mut bound.state)
            .map_err(|e| Failure::Probe(e.to_string()))?;
        if done(&bound.state) {
            return Ok(());
        }
        bound
            .queue
            .flush()
            .map_err(|e| Failure::Probe(e.to_string()))?;
        let Some(guard) = bound.queue.prepare_read() else {
            continue;
        };
        let wire = guard.connection_fd();
        let mut fds = [PollFd::new(&wire, PollFlags::IN)];
        let two_seconds = Timespec {
            tv_sec: 2,
            tv_nsec: 0,
        };
        let ready = rustix::event::poll(&mut fds, Some(&two_seconds))
            .map_err(|e| Failure::Probe(e.to_string()))?;
        if ready > 0 {
            guard.read().map_err(|e| Failure::Probe(e.to_string()))?;
        }
    }
    Err(Failure::Probe(format!("timed out waiting for {what}")))
}

/// A `width` x `height` ARGB8888 buffer holding a coordinate-encoding pattern.
fn buffer(bound: &Bound, width: u32, height: u32) -> Result<(WlBuffer, File), Failure> {
    let size = PhysicalSize {
        w: width,
        h: height,
    };
    let bytes = Pattern::CoordinateEncoding.bytes(size);
    let fd = memfd_create("casement-probe", MemfdFlags::CLOEXEC)
        .map_err(|e| Failure::Probe(e.to_string()))?;
    ftruncate(&fd, bytes.len() as u64).map_err(|e| Failure::Probe(e.to_string()))?;
    let file = File::from(fd);
    file.write_all_at(&bytes, 0)
        .map_err(|e| Failure::Probe(e.to_string()))?;
    let pool = bound
        .shm
        .create_pool(file.as_fd(), bytes.len() as i32, &bound.qh, ());
    let buffer = pool.create_buffer(
        0,
        width as i32,
        height as i32,
        (width * 4) as i32,
        Format::Argb8888,
        &bound.qh,
        (),
    );
    pool.destroy();
    Ok((buffer, file))
}

/// `--name value` from `args`.
fn option<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == name)
        .and_then(|at| args.get(at + 1))
        .map(String::as_str)
}

fn size_option(args: &[String]) -> (u32, u32) {
    option(args, "--size")
        .and_then(|text| text.split_once('x'))
        .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))
        .unwrap_or(DEFAULT_SIZE)
}

/// Keeps dispatching until the compositor closes the surface (or forever).
fn hold(bound: &mut Bound) -> Result<(), Failure> {
    while !bound.state.closed {
        match wait_for(bound, "a close", |state| state.closed) {
            Err(Failure::Probe(why)) if why.starts_with("timed out") => {}
            other => other?,
        }
    }
    println!("{}", json!({ "event": "closed" }));
    Ok(())
}

/// The flags a probe takes: those that name a value after them, and those that stand alone.
pub(crate) struct Flags {
    valued: &'static [&'static str],
    alone: &'static [&'static str],
}

/// What `toplevel` takes.
pub(crate) const TOPLEVEL_FLAGS: Flags = Flags {
    valued: &["--title", "--app-id", "--size"],
    alone: &["--hold", "--maximised"],
};

/// What `layer` takes.
pub(crate) const LAYER_FLAGS: Flags = Flags {
    valued: &["--layer", "--namespace", "--size"],
    alone: &["--hold"],
};

impl Flags {
    /// `args` when every one is a flag of this probe (with its value, where it takes one);
    /// otherwise a usage error: a misspelt flag is never ignored.
    pub(crate) fn check(&self, args: &[String]) -> Result<(), Failure> {
        let mut rest = args.iter();
        while let Some(arg) = rest.next() {
            let known = match arg.as_str() {
                flag if self.valued.contains(&flag) => rest.next().is_some(),
                flag => self.alone.contains(&flag),
            };
            if !known {
                return Err(Failure::Usage);
            }
        }
        Ok(())
    }
}

/// The states in a configure's wire array of native-endian `u32`s (values this protocol version
/// does not know are left out).
fn states_of(words: &[u8]) -> Vec<xdg_toplevel::State> {
    words
        .chunks_exact(4)
        .filter_map(|word| {
            let value = u32::from_ne_bytes([word[0], word[1], word[2], word[3]]);
            xdg_toplevel::State::try_from(value).ok()
        })
        .collect()
}

/// Maps an xdg toplevel and prints its configure.
pub(crate) fn toplevel(args: &[String]) -> Result<(), Failure> {
    TOPLEVEL_FLAGS.check(args)?;
    let (_conn, mut bound) = bind()?;
    let wm_base: XdgWmBase = bind_in(&bound.globals, &bound.qh, 1..=5)?;
    let surface = bound.compositor.create_surface(&bound.qh, ());
    let xdg = wm_base.get_xdg_surface(&surface, &bound.qh, ());
    let toplevel = xdg.get_toplevel(&bound.qh, ());
    toplevel.set_title(
        option(args, "--title")
            .unwrap_or("casement-probe")
            .to_owned(),
    );
    toplevel.set_app_id(
        option(args, "--app-id")
            .unwrap_or("casement-probe")
            .to_owned(),
    );
    if args.iter().any(|a| a == "--maximised") {
        toplevel.set_maximized();
    }
    surface.commit();
    wait_for(&mut bound, "the first configure", |state| state.acked)?;
    let (cw, ch) = bound.state.configured.unwrap_or((0, 0));
    let maximised = bound.state.states.contains(&xdg_toplevel::State::Maximized);
    println!(
        "{}",
        json!({ "event": "configure", "width": cw, "height": ch, "maximised": maximised })
    );
    let wanted = size_option(args);
    let (width, height) = if cw == 0 || ch == 0 { wanted } else { (cw, ch) };
    let (pixels, _keep) = buffer(&bound, width, height)?;
    surface.attach(Some(&pixels), 0, 0);
    surface.commit();
    bound
        .queue
        .roundtrip(&mut bound.state)
        .map_err(|e| Failure::Probe(e.to_string()))?;
    println!(
        "{}",
        json!({ "event": "mapped", "width": width, "height": height })
    );
    if args.iter().any(|a| a == "--hold") {
        hold(&mut bound)?;
    }
    Ok(())
}

/// Maps a layer surface and prints its configure.
pub(crate) fn layer(args: &[String]) -> Result<(), Failure> {
    LAYER_FLAGS.check(args)?;
    let (_conn, mut bound) = bind()?;
    let shell: ZwlrLayerShellV1 = bind_in(&bound.globals, &bound.qh, 1..=4)?;
    let which = match option(args, "--layer") {
        Some("background") => Layer::Background,
        Some("bottom") => Layer::Bottom,
        Some("overlay") => Layer::Overlay,
        _ => Layer::Top,
    };
    let surface = bound.compositor.create_surface(&bound.qh, ());
    let namespace = option(args, "--namespace")
        .unwrap_or("casement-probe")
        .to_owned();
    let layer = shell.get_layer_surface(&surface, None, which, namespace, &bound.qh, ());
    let (width, height) = size_option(args);
    layer.set_size(width, height);
    layer.set_anchor(Anchor::Top | Anchor::Left);
    surface.commit();
    wait_for(&mut bound, "the first configure", |state| state.acked)?;
    let (cw, ch) = bound.state.configured.unwrap_or((0, 0));
    println!(
        "{}",
        json!({ "event": "configure", "width": cw, "height": ch })
    );
    let (w, h) = if cw == 0 || ch == 0 {
        (width, height)
    } else {
        (cw, ch)
    };
    let (pixels, _keep) = buffer(&bound, w, h)?;
    surface.attach(Some(&pixels), 0, 0);
    surface.commit();
    bound
        .queue
        .roundtrip(&mut bound.state)
        .map_err(|e| Failure::Probe(e.to_string()))?;
    println!("{}", json!({ "event": "mapped", "width": w, "height": h }));
    if args.iter().any(|a| a == "--hold") {
        hold(&mut bound)?;
    }
    Ok(())
}

impl Dispatch<WlRegistry, GlobalListContents> for State {
    fn event(
        _: &mut Self,
        _: &WlRegistry,
        _: wayland_client::protocol::wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

macro_rules! ignore_events {
    ($($proxy:ty => $event:ty),* $(,)?) => {
        $(
            impl Dispatch<$proxy, ()> for State {
                fn event(_: &mut Self, _: &$proxy, _: $event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
            }
        )*
    };
}

ignore_events! {
    WlCompositor => wayland_client::protocol::wl_compositor::Event,
    WlShm => wl_shm::Event,
    WlShmPool => wl_shm_pool::Event,
    WlBuffer => wl_buffer::Event,
    WlSurface => wl_surface::Event,
    ZwlrLayerShellV1 => zwlr_layer_shell_v1::Event,
}

impl Dispatch<XdgWmBase, ()> for State {
    fn event(
        _: &mut Self,
        base: &XdgWmBase,
        event: xdg_wm_base::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            base.pong(serial);
        }
    }
}

impl Dispatch<XdgSurface, ()> for State {
    fn event(
        state: &mut Self,
        surface: &XdgSurface,
        event: xdg_surface::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_surface::Event::Configure { serial } = event {
            surface.ack_configure(serial);
            state.configured.get_or_insert((0, 0));
            state.acked = true;
        }
    }
}

impl Dispatch<XdgToplevel, ()> for State {
    fn event(
        state: &mut Self,
        _: &XdgToplevel,
        event: xdg_toplevel::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            xdg_toplevel::Event::Configure {
                width,
                height,
                states,
            } => {
                state.configured = Some((width.max(0) as u32, height.max(0) as u32));
                state.states = states_of(&states);
            }
            xdg_toplevel::Event::Close => state.closed = true,
            _ => {}
        }
    }
}

impl Dispatch<ZwlrLayerSurfaceV1, ()> for State {
    fn event(
        state: &mut Self,
        surface: &ZwlrLayerSurfaceV1,
        event: zwlr_layer_surface_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_layer_surface_v1::Event::Configure {
                serial,
                width,
                height,
            } => {
                surface.ack_configure(serial);
                state.configured = Some((width, height));
                state.acked = true;
            }
            zwlr_layer_surface_v1::Event::Closed => state.closed = true,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(text: &str) -> Vec<String> {
        text.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn known_flags_pass_and_unknown_ones_are_usage_errors() {
        let cases = [
            ("toplevel: none", &TOPLEVEL_FLAGS, "", true),
            (
                "toplevel: every flag",
                &TOPLEVEL_FLAGS,
                "--title T --app-id A --size 10x10 --hold --maximised",
                true,
            ),
            (
                "toplevel: an unknown flag",
                &TOPLEVEL_FLAGS,
                "--maximized",
                false,
            ),
            ("toplevel: a stray word", &TOPLEVEL_FLAGS, "hold", false),
            (
                "toplevel: a value flag with no value",
                &TOPLEVEL_FLAGS,
                "--title",
                false,
            ),
            (
                "layer: every flag",
                &LAYER_FLAGS,
                "--layer top --namespace n --size 1x1 --hold",
                true,
            ),
            ("layer: a toplevel flag", &LAYER_FLAGS, "--maximised", false),
        ];
        for (name, flags, text, passes) in cases {
            assert_eq!(flags.check(&args(text)).is_ok(), passes, "{name}");
        }
    }

    #[test]
    fn states_are_read_from_the_wire_array() {
        let wire: Vec<u8> = [1_u32, 4, 999]
            .into_iter()
            .flat_map(u32::to_ne_bytes)
            .collect();
        assert_eq!(
            states_of(&wire),
            vec![
                xdg_toplevel::State::Maximized,
                xdg_toplevel::State::Activated
            ]
        );
        assert_eq!(states_of(&[]), Vec::new());
    }
}
