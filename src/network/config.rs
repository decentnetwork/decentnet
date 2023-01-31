use std::time::Duration;

use libp2p::{core::connection::ListenerId, rendezvous::Cookie};

use crate::network::BootNode;

pub const RENDEZVOUS_NAMESPACE: &str = "decentnet";
pub const IDENTIFY_PROTOCOL_VERSION: &str = "decentnet/0.1.0";

#[derive(Clone)]
pub struct NetworkConfig {
    pub server_mode: bool,
    pub debug_mode: bool,
    pub has_ipv6_support: bool,
    pub boot_nodes: Vec<BootNode>,
    pub default_port: u16,
    pub ping_interval: Duration,
    pub enable_identify: bool,
    pub live_connection: bool,
    pub local_discovery: bool,
    pub use_yamux: bool,
    pub rendezvous_cookie: Option<Cookie>,
    pub relay_dial_mode: bool,
    pub relay_id: Option<String>,
    pub listener_ids: Vec<ListenerId>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            server_mode: false,
            debug_mode: false,
            has_ipv6_support: false,
            boot_nodes: vec![],
            default_port: 26117,
            ping_interval: Duration::from_secs(60),
            live_connection: false,
            local_discovery: false,
            use_yamux: false,
            enable_identify: true,
            rendezvous_cookie: None,
            relay_dial_mode: false,
            listener_ids: vec![],
            relay_id: None,
        }
    }
}

impl NetworkConfig {
    pub fn rendezvous_cookie_mut(&mut self) -> &mut Option<Cookie> {
        &mut self.rendezvous_cookie
    }
}
