//! Every element casement draws, as one enum that smithay cannot draw on its own.
//!
//! It is smithay's [`Element`] (where it is, what changed), not its `RenderElement`: a
//! clipped window, a genie strip or a blur drawn by smithay directly falls back to its plain
//! surface, which is how windows lost their rounded corners on real hardware while every
//! headless picture had them (casement 4e769bf). The only drawable form is
//! [`crate::Routed`] (see [`crate::routed`]), which sends those variants to the renderer's
//! [`crate::Compositor`] hooks; the plain draws below are crate-private, for it alone.

use super::{
    BlurV0Element, ClipStripElement, LockCoverElement, RoundedClipElement, SharedElement,
    StripElement,
};
use smithay::backend::renderer::element::memory::MemoryRenderBufferRenderElement;
use smithay::backend::renderer::element::solid::SolidColorRenderElement;
use smithay::backend::renderer::element::surface::WaylandSurfaceRenderElement;
use smithay::backend::renderer::element::texture::TextureRenderElement;
use smithay::backend::renderer::element::utils::RescaleRenderElement;
use smithay::backend::renderer::element::{Element, Id, Kind, RenderElement, UnderlyingStorage};
use smithay::backend::renderer::utils::{CommitCounter, DamageSet, OpaqueRegions};
use smithay::backend::renderer::{ImportAll, ImportMem, Renderer, RendererSuper, Texture};
use smithay::utils::user_data::UserDataMap;
use smithay::utils::{Buffer as BufferCoords, Physical, Point, Rectangle, Scale, Transform};

/// Every element casement draws.
pub enum CasementElement<R>
where
    R: Renderer + ImportAll + ImportMem,
    R::TextureId: Texture + 'static,
{
    /// A client surface (window, layer, popup, lock surface, cursor surface).
    Surface(WaylandSurfaceRenderElement<R>),
    /// A solid rectangle.
    Solid(SolidColorRenderElement),
    /// A CPU raster (chrome, the cursor theme).
    Memory(MemoryRenderBufferRenderElement<R>),
    /// A GPU image kept by the compositor (a closing window's last image).
    Texture(TextureRenderElement<<R as RendererSuper>::TextureId>),
    /// The P0 behind-surface blur (ext-background-effect).
    Blur(BlurV0Element),
    /// The cover over a locked output without a live lock surface.
    Cover(LockCoverElement),
    /// A window's surface clipped at its corners (GLES shader; pieces elsewhere).
    Clip(RoundedClipElement<R>),
    /// One strip of a genie's client surface.
    SurfaceStrip(StripElement<SharedElement<WaylandSurfaceRenderElement<R>>>),
    /// One strip of a genie's frame (titlebar and title).
    MemoryStrip(StripElement<SharedElement<MemoryRenderBufferRenderElement<R>>>),
    /// One strip of a genie's client surface that the window's rounded clip reaches.
    ClipStrip(ClipStripElement<R>),
    /// A client surface stretched about the window's corner: a window (or a menu of it) in a
    /// Scale flight.
    Stretched(RescaleRenderElement<WaylandSurfaceRenderElement<R>>),
}

/// `$body` with `$e` bound to the variant's element, whichever it is.
macro_rules! each {
    ($value:expr, $e:ident => $body:expr) => {
        match $value {
            CasementElement::Surface($e) => $body,
            CasementElement::Solid($e) => $body,
            CasementElement::Memory($e) => $body,
            CasementElement::Texture($e) => $body,
            CasementElement::Blur($e) => $body,
            CasementElement::Cover($e) => $body,
            CasementElement::Clip($e) => $body,
            CasementElement::SurfaceStrip($e) => $body,
            CasementElement::MemoryStrip($e) => $body,
            CasementElement::ClipStrip($e) => $body,
            CasementElement::Stretched($e) => $body,
        }
    };
}

/// `From` each element type, as `render_elements!` gave.
macro_rules! from {
    ($($variant:ident: $inner:ty),* $(,)?) => {
        $(
            impl<R> From<$inner> for CasementElement<R>
            where
                R: Renderer + ImportAll + ImportMem,
                R::TextureId: Texture + 'static,
            {
                fn from(element: $inner) -> CasementElement<R> {
                    CasementElement::$variant(element)
                }
            }
        )*
    };
}

from! {
    Surface: WaylandSurfaceRenderElement<R>,
    Solid: SolidColorRenderElement,
    Memory: MemoryRenderBufferRenderElement<R>,
    Texture: TextureRenderElement<<R as RendererSuper>::TextureId>,
    Blur: BlurV0Element,
    Cover: LockCoverElement,
    Clip: RoundedClipElement<R>,
    SurfaceStrip: StripElement<SharedElement<WaylandSurfaceRenderElement<R>>>,
    MemoryStrip: StripElement<SharedElement<MemoryRenderBufferRenderElement<R>>>,
    ClipStrip: ClipStripElement<R>,
    Stretched: RescaleRenderElement<WaylandSurfaceRenderElement<R>>,
}

impl<R> std::fmt::Debug for CasementElement<R>
where
    R: Renderer + ImportAll + ImportMem,
    R::TextureId: Texture + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let variant = match self {
            CasementElement::Surface(_) => "Surface",
            CasementElement::Solid(_) => "Solid",
            CasementElement::Memory(_) => "Memory",
            CasementElement::Texture(_) => "Texture",
            CasementElement::Blur(_) => "Blur",
            CasementElement::Cover(_) => "Cover",
            CasementElement::Clip(_) => "Clip",
            CasementElement::SurfaceStrip(_) => "SurfaceStrip",
            CasementElement::MemoryStrip(_) => "MemoryStrip",
            CasementElement::ClipStrip(_) => "ClipStrip",
            CasementElement::Stretched(_) => "Stretched",
        };
        f.debug_tuple(variant).field(self.id()).finish()
    }
}

impl<R> Element for CasementElement<R>
where
    R: Renderer + ImportAll + ImportMem,
    R::TextureId: Texture + 'static,
{
    fn id(&self) -> &Id {
        each!(self, e => e.id())
    }

    fn current_commit(&self) -> CommitCounter {
        each!(self, e => e.current_commit())
    }

    fn location(&self, scale: Scale<f64>) -> Point<i32, Physical> {
        each!(self, e => e.location(scale))
    }

    fn src(&self) -> Rectangle<f64, BufferCoords> {
        each!(self, e => e.src())
    }

    fn transform(&self) -> Transform {
        each!(self, e => e.transform())
    }

    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        each!(self, e => e.geometry(scale))
    }

    fn damage_since(
        &self,
        scale: Scale<f64>,
        commit: Option<CommitCounter>,
    ) -> DamageSet<i32, Physical> {
        each!(self, e => e.damage_since(scale, commit))
    }

    fn opaque_regions(&self, scale: Scale<f64>) -> OpaqueRegions<i32, Physical> {
        each!(self, e => e.opaque_regions(scale))
    }

    fn alpha(&self) -> f32 {
        each!(self, e => e.alpha())
    }

    fn kind(&self) -> Kind {
        each!(self, e => e.kind())
    }

    fn is_framebuffer_effect(&self) -> bool {
        each!(self, e => e.is_framebuffer_effect())
    }
}

impl<R> CasementElement<R>
where
    R: Renderer + ImportAll + ImportMem,
    R::TextureId: Texture + 'static,
{
    /// The element's own draw, with no hook: for [`crate::Routed`] alone.
    pub(crate) fn draw_plain(
        &self,
        frame: &mut R::Frame<'_, '_>,
        src: Rectangle<f64, BufferCoords>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        opaque_regions: &[Rectangle<i32, Physical>],
        cache: Option<&UserDataMap>,
    ) -> Result<(), R::Error> {
        each!(self, e => RenderElement::<R>::draw(e, frame, src, dst, damage, opaque_regions, cache))
    }

    /// The buffer behind the element, for direct scanout: for [`crate::Routed`] alone.
    pub(crate) fn storage(&self, renderer: &mut R) -> Option<UnderlyingStorage<'_>> {
        each!(self, e => RenderElement::<R>::underlying_storage(e, renderer))
    }

    /// The element's own framebuffer capture: for [`crate::Routed`] alone.
    pub(crate) fn capture_plain(
        &self,
        frame: &mut R::Frame<'_, '_>,
        src: Rectangle<f64, BufferCoords>,
        dst: Rectangle<i32, Physical>,
        cache: &UserDataMap,
    ) -> Result<(), R::Error> {
        each!(self, e => RenderElement::<R>::capture_framebuffer(e, frame, src, dst, cache))
    }
}
