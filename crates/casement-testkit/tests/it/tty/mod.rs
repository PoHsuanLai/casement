//! The tty backend over its fakes (P1-D1 to P1-D3, P1-T): session, hotplug, KMS and scanout, with
//! no device; `integration` runs the whole `State` over them.

mod integration;
mod kms;
mod scanout;
mod session;
