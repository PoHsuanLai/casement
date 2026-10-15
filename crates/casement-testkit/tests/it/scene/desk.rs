//! A desk: one fixture, one public client, and the helpers that put surfaces on it the way a
//! toolkit would (a toplevel at the output's scale, a popup, a layer surface, a lock).

use casement_core::geometry::{LogicalPoint, PhysicalSize};
use casement_core::input::DeviceClass;
use casement_core::policy::ConnectionRole;
use casement_core::{OutputName, WindowId};
use casement_testclient::misc::decoration::{Decoration, DecorationManager};
use casement_testclient::misc::viewporter::Viewporter;
use casement_testclient::surface::layer::{
    Anchor, KeyboardInteractivity, Layer, LayerAsk, LayerSurface,
};
use casement_testclient::surface::popup::{Popup, PopupSpec};
use casement_testclient::surface::subsurface::Surface;
use casement_testclient::surface::toplevel::{Toplevel, WmBase};
use casement_testclient::{Pattern, shm_buffer};
use casement_testkit::{ClientId, Fixture, FrameCount, ShotTarget, TestOrigin};
use std::any::Any;
use std::time::Duration;
use wayland_client::protocol::wl_surface::WlSurface;

/// The output every scene is shot from.
pub const OUTPUT: &str = "HEADLESS-1";

/// How a client draws for the output: its buffer scale in 120ths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dpi {
    /// 1x: buffer pixels are logical pixels.
    One,
    /// 1.5x: a buffer of 1.5 times the size under a viewport destination.
    OneAndAHalf,
    /// 2x: a buffer of twice the size under `wl_surface.set_buffer_scale(2)`.
    Two,
}

impl Dpi {
    /// The buffer's size in pixels for a surface of `logical` size.
    pub fn buffer(self, logical: (u32, u32)) -> PhysicalSize {
        let times = |n: u32, num: u32, den: u32| n * num / den;
        let (num, den) = match self {
            Dpi::One => (1, 1),
            Dpi::OneAndAHalf => (3, 2),
            Dpi::Two => (2, 1),
        };
        PhysicalSize {
            w: times(logical.0, num, den),
            h: times(logical.1, num, den),
        }
    }
}

/// What a scene keeps alive until it has been shot (dropping a proxy must not matter, but a
/// buffer a surface still shows should not go away).
#[derive(Default)]
pub struct Kept(Vec<Box<dyn Any>>);

impl Kept {
    /// Keeps `thing`.
    pub fn keep(&mut self, thing: impl Any) {
        self.0.push(Box::new(thing));
    }
}

/// One desktop with one connected client and its `xdg_wm_base`.
pub struct Desk<'a> {
    /// The compositor.
    pub fixture: &'a mut Fixture,
    /// The client.
    pub id: ClientId,
    /// Its window manager base.
    pub base: WmBase,
    /// What stays alive.
    pub kept: Kept,
    /// The decoration object of each framed window, by its surface: a window must lose it
    /// before it goes (`Desk::close`).
    decorations: Vec<(WlSurface, Decoration)>,
}

impl<'a> Desk<'a> {
    /// Connects a public client to `fixture`.
    pub fn at(fixture: &'a mut Fixture) -> Desk<'a> {
        // The compositor starts the pointer at the output's centre; the goldens were shot with
        // the cursor in the corner, out of every scene's way.
        let device = fixture.input().plug(
            DeviceClass::Pointer,
            TestOrigin::Physical(DeviceClass::Pointer),
        );
        fixture
            .input()
            .pointer_to(device, LogicalPoint { x: 0, y: 0 });
        let id = fixture.connect(ConnectionRole::Public);
        fixture.pump();
        let base = WmBase::bind(fixture.client(id), 6);
        Desk {
            fixture,
            id,
            base,
            kept: Kept::default(),
            decorations: Vec::new(),
        }
    }

    /// Maps a toplevel of `logical` size showing `pattern`, drawn for `dpi`.
    pub fn window(
        &mut self,
        app_id: &str,
        logical: (u32, u32),
        pattern: Pattern,
        dpi: Dpi,
    ) -> Toplevel {
        let surface = self
            .fixture
            .client(self.id)
            .create_surface()
            .expect("a surface");
        self.window_on(surface, app_id, logical, pattern, dpi)
    }

    /// As [`Desk::window`], on a surface the caller already gave requests to.
    pub fn window_on(
        &mut self,
        surface: WlSurface,
        app_id: &str,
        logical: (u32, u32),
        pattern: Pattern,
        dpi: Dpi,
    ) -> Toplevel {
        self.window_framed(surface, app_id, None, logical, pattern, dpi)
    }

    /// As [`Desk::window`], titled `title` and answered server-side: casement draws its frame
    /// (titlebar, lights, rounded corners and shadow) round the `logical`-sized content.
    pub fn framed_window(
        &mut self,
        app_id: &str,
        title: &str,
        logical: (u32, u32),
        pattern: Pattern,
        dpi: Dpi,
    ) -> Toplevel {
        let surface = self
            .fixture
            .client(self.id)
            .create_surface()
            .expect("a surface");
        self.window_framed(surface, app_id, Some(title), logical, pattern, dpi)
    }

    /// Maps a toplevel on `surface`; with a `title` it asks for the server-side frame first.
    fn window_framed(
        &mut self,
        surface: WlSurface,
        app_id: &str,
        title: Option<&str>,
        logical: (u32, u32),
        pattern: Pattern,
        dpi: Dpi,
    ) -> Toplevel {
        let top = self.base.toplevel_on(self.fixture.client(self.id), surface);
        top.set_app_id(app_id);
        if let Some(title) = title {
            top.set_title(title);
            let client = self.fixture.client(self.id);
            let manager = DecorationManager::bind(client).expect("xdg_decoration");
            let decoration = manager.decorate(client, &top);
            self.kept.keep(manager);
            self.decorations.push((top.surface.clone(), decoration));
        }
        top.commit_bare();
        self.fixture.pump();
        assert!(top.ack_latest().is_some(), "the initial configure arrives");
        let client = self.fixture.client(self.id);
        let buffer = shm_buffer(client, dpi.buffer(logical), pattern);
        self.scale_to(&top.surface, logical, dpi);
        top.show(&buffer);
        self.fixture.pump();
        self.kept.keep(buffer);
        top
    }

    /// Tells the compositor how `surface`'s buffer relates to its logical size.
    fn scale_to(&mut self, surface: &WlSurface, logical: (u32, u32), dpi: Dpi) {
        match dpi {
            Dpi::One => {}
            Dpi::Two => surface.set_buffer_scale(2),
            Dpi::OneAndAHalf => {
                let client = self.fixture.client(self.id);
                let wrapped = Surface {
                    surface: surface.clone(),
                };
                let viewport = Viewporter::bind(client).viewport(client, &wrapped);
                viewport.destination(
                    i32::try_from(logical.0).expect("a small width"),
                    i32::try_from(logical.1).expect("a small height"),
                );
                self.kept.keep(viewport);
            }
        }
    }

    /// Maps a popup on `parent` placed by `spec`, filled with `pattern`.
    pub fn popup(&mut self, parent: &Toplevel, spec: PopupSpec, pattern: Pattern, dpi: Dpi) {
        let client = self.fixture.client(self.id);
        let surface = client.create_surface().expect("a surface");
        let popup = Popup::of(client, &self.base, parent, surface, spec);
        popup.surface.commit();
        self.fixture.pump();
        assert!(popup.ack_latest().is_some(), "the popup is configured");
        let logical = (spec.size.0.unsigned_abs(), spec.size.1.unsigned_abs());
        let buffer = shm_buffer(self.fixture.client(self.id), dpi.buffer(logical), pattern);
        self.scale_to(&popup.surface, logical, dpi);
        popup.surface.attach(Some(buffer.buffer()), 0, 0);
        popup.surface.damage_buffer(0, 0, i32::MAX, i32::MAX);
        popup.surface.commit();
        self.fixture.pump();
        self.kept.keep(buffer);
        self.kept.keep(popup);
    }

    /// Spreads `colour` (ARGB) over the whole output as a wallpaper (a background layer surface
    /// that reserves nothing), drawn for `dpi`. The bare desktop is black, and a shadow is
    /// black: only a lighter ground shows one.
    pub fn wallpaper(&mut self, colour: u32, dpi: Dpi) {
        let ask = LayerAsk {
            level: Layer::Background,
            namespace: "wallpaper",
            anchor: Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
            exclusive_zone: -1,
            keyboard: KeyboardInteractivity::None,
            size: (0, 0),
        };
        let layer = LayerSurface::create(self.fixture.client(self.id), &ask).expect("a layer");
        self.fixture.pump();
        let configure = layer.configure().expect("the wallpaper is sized");
        assert!(layer.ack(self.fixture.client(self.id)));
        let logical = (configure.width, configure.height);
        let buffer = shm_buffer(
            self.fixture.client(self.id),
            dpi.buffer(logical),
            Pattern::Solid(colour),
        );
        self.scale_to(layer.surface(), logical, dpi);
        layer.show(self.fixture.client(self.id), &buffer);
        self.fixture.pump();
        self.kept.keep((layer, buffer));
    }

    /// Closes `top` the way a toolkit does: its decoration object first (xdg-decoration says it
    /// must be destroyed before the toplevel, and the compositor ends the client's connection,
    /// every surface of it, if it is not), then the toplevel.
    pub fn close(&mut self, top: &Toplevel) {
        let (gone, kept): (Vec<_>, Vec<_>) = std::mem::take(&mut self.decorations)
            .into_iter()
            .partition(|(surface, _)| *surface == top.surface);
        self.decorations = kept;
        for (_, decoration) in gone {
            decoration.decoration.destroy();
        }
        top.destroy();
    }

    /// Plugs a pointer and puts it at `at` (logical).
    pub fn pointer_at(&mut self, at: LogicalPoint) {
        let device = self.fixture.input().plug(
            DeviceClass::Pointer,
            TestOrigin::Physical(DeviceClass::Pointer),
        );
        self.fixture.input().pointer_to(device, at);
    }

    /// Another client of the same compositor.
    pub fn another(&mut self) -> ClientId {
        let id = self.fixture.connect(ConnectionRole::Public);
        self.fixture.pump();
        id
    }

    /// Lets the compositor present, then shoots the output.
    pub fn shoot(&mut self) -> image::RgbaImage {
        shoot_output(self.fixture)
    }
}

/// How long the scene's motion (a window opening) takes to settle, with room to spare.
const SETTLE: Duration = Duration::from_secs(2);

/// Presents a few frames and shoots the output.
pub fn shoot_output(fixture: &mut Fixture) -> image::RgbaImage {
    let output = OutputName::parse(OUTPUT).expect("a valid output name");
    fixture.advance(SETTLE);
    fixture.frames(&output, FrameCount(2));
    fixture.shot(ShotTarget::Output(output))
}

/// The model's only window.
pub fn only_window(fixture: &Fixture) -> WindowId {
    *fixture
        .model()
        .windows()
        .keys()
        .next()
        .expect("a mapped window")
}
