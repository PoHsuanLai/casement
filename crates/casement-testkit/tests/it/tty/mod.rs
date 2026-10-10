//! The tty backend over its fakes (P1-D1 to P1-D3): session, hotplug, KMS and scanout, with no
//! device. Every test is ignored until its lane fills the bodies; the lanes add rows.

mod kms;
mod scanout;
mod session;
