//! Giving a fuzz client something to fuzz: every advertised global bound, then a few objects of
//! the shapes most requests need (surfaces, a region, a seat's devices, an xdg toplevel).

use super::host::FuzzHost;
use crate::fixture::ClientId;
use casement_testclient::TestClient;

/// One creating request of the priming script: the interface of the object it goes to, its
/// name, and the integers its arguments are made from.
const SCRIPT: [(&str, &str, [i32; 4]); 12] = [
    ("wl_compositor", "create_surface", [1, 0, 1, 0]),
    ("wl_compositor", "create_surface", [1, 1, 1, 1]),
    ("wl_compositor", "create_region", [1, 0, 1, 0]),
    ("xdg_wm_base", "create_positioner", [1, 0, 1, 0]),
    ("xdg_wm_base", "get_xdg_surface", [1, 0, 1, 0]),
    ("xdg_surface", "get_toplevel", [1, 0, 1, 0]),
    ("wl_seat", "get_pointer", [1, 0, 1, 0]),
    ("wl_seat", "get_keyboard", [1, 0, 1, 0]),
    ("wl_seat", "get_touch", [1, 0, 1, 0]),
    ("wl_data_device_manager", "create_data_source", [1, 0, 1, 0]),
    ("wl_data_device_manager", "get_data_device", [1, 0, 1, 0]),
    ("wl_subcompositor", "get_subsurface", [1, 1, 1, 0]),
];

/// Binds every global `id` was offered and runs the script; stops quietly where an object it
/// needs is missing (a sandboxed client has no layer shell, say).
pub(super) fn prime<H: FuzzHost>(host: &mut H, id: ClientId) {
    bind_all(host.client(id));
    host.pump();
    for (interface, request, ints) in SCRIPT {
        let client = host.client(id);
        let (Some(target), Some(opcode)) = (
            client
                .live_objects()
                .into_iter()
                .find(|(_, name)| name == interface)
                .map(|(object, _)| object),
            TestClient::request_opcode(interface, request),
        ) else {
            continue;
        };
        client.send_wire(target, interface, opcode, &ints);
        host.pump();
        if !host.client(id).is_connected() {
            return;
        }
    }
    leave_a_destroyed_object(host, id);
}

/// A region made and destroyed at once, so `ObjectRef::Destroyed` has an id to name.
fn leave_a_destroyed_object<H: FuzzHost>(host: &mut H, id: ClientId) {
    let client = host.client(id);
    let compositor = client
        .live_objects()
        .into_iter()
        .find(|(_, name)| name == "wl_compositor")
        .map(|(object, _)| object);
    let (Some(compositor), Some(create), Some(destroy)) = (
        compositor,
        TestClient::request_opcode("wl_compositor", "create_region"),
        TestClient::request_opcode("wl_region", "destroy"),
    ) else {
        return;
    };
    client.send_wire(compositor, "wl_compositor", create, &[1]);
    let region = client
        .live_objects()
        .into_iter()
        .filter(|(_, name)| name == "wl_region")
        .map(|(object, _)| object)
        .max();
    if let Some(region) = region {
        client.send_wire(region, "wl_region", destroy, &[]);
    }
    host.pump();
}

/// `wl_registry.bind` for each advertised global, at its advertised version.
fn bind_all(client: &mut TestClient) {
    let Some(registry) = client
        .live_objects()
        .into_iter()
        .find(|(_, name)| name == "wl_registry")
        .map(|(object, _)| object)
    else {
        return;
    };
    let Some(bind) = TestClient::request_opcode("wl_registry", "bind") else {
        return;
    };
    let advertised = client.globals().contents().clone_list().len();
    for at in 0..advertised {
        // The first integer picks the global, the second says "advertised version".
        let at = i32::try_from(at).unwrap_or(i32::MAX);
        client.send_wire(registry, "wl_registry", bind, &[at, 0]);
    }
}
