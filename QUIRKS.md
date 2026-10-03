# Quirks

Behaviour facts about hardware, drivers and toolkits that casement works around, in our own
words (CONVENTIONS addendum 1). One row per fact; a fact without a test or a fixture is a
rumour and does not go here.

| Fact (keyed by driver, PCI class, connector type or toolkit) | Behaviour | Link | Test or fixture |
| --- | --- | --- | --- |
| smithay 118e34ff `renderer_pixman` with `renderer_multi` | every `PixmanRenderer` shares one static `ContextId`, so two pixman devices collapse into one `MultiTexture` slot and the cross-device copy unwraps `None` on the first frame | not filed upstream (A13); spike SP4 (`~/rs-wt/spikes/casement-sp4`, `cross plain direct`) | `casement_backend::gpu::SoftPixman` (own `ContextId`); P1-D4's soft topology suite |
| smithay 118e34ff `UdmabufAllocator::create_buffer` | the stride is width x bits per pixel (8x over-allocation): above about 1920x1080 the kernel's 64 MB `udmabuf.size_limit_mb` refuses with EINVAL and smithay silently falls back from the dma shadow copy to a CPU copy | not filed upstream (A13); SP4 `alloc` | `casement_backend::gpu::FixedUdmabuf`; boundary row A4 bans the call |
