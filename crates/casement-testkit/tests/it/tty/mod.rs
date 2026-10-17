//! The tty backend over its fakes (P1-D1 to P1-D3, P1-T): session, hotplug, KMS and scanout, with
//! no device; `integration` runs the whole `State` over them.

mod capture_gpu;
mod direct;
mod firmware;
mod integration;
mod kms;
mod recovery;
mod scanout;
mod session;
mod software;
