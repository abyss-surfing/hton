//! HTON domain owners; entry points inject resource and authorization capabilities.
#![forbid(unsafe_code)]

pub mod apply;
pub mod backup;
pub mod catalog;
pub mod certificates;
pub mod checks;
pub mod config;
pub mod exports;
pub mod host;
pub mod host_crypto;
pub mod identities;
pub mod journal;
pub mod lifecycle;
pub mod lock;
pub mod plan;
pub mod port_allocation;
pub mod registry;
pub mod render;
pub mod secrets;
pub mod selection;
pub mod setup;

#[path = "../../adapters/engines/caddy.rs"]
pub mod caddy_adapter;
#[path = "../../adapters/foreign.rs"]
pub mod foreign_adapter;
#[path = "../../adapters/engines/haproxy.rs"]
pub mod haproxy_adapter;
#[path = "../../adapters/resources.rs"]
pub mod resource_adapter;
#[path = "../../../services/onomat/runtime/dns/rust/unbound.rs"]
pub mod unbound_adapter;
#[path = "../../adapters/engines/xray.rs"]
pub mod xray_adapter;
