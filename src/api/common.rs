use crate::network::{
    behaviour::DecentNetworkBehaviour, config::NetworkConfig, identity::Network, IdentityImpl,
    NetworkNode,
};

use libp2p::{
    futures::{
        channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender},
        stream::FusedStream,
        Stream, StreamExt,
    },
    identity::ed25519::SecretKey,
    PeerId, Swarm,
};

use std::{str::FromStr, task::Poll, time::Duration};
use tokio::runtime::Runtime;

use super::lib::load_bootnodes;

static mut RUNTIME: Option<Runtime> = None;
static mut NETWORK: Option<Network> = None;
static mut NETWORK_CONFIG: Option<NetworkConfig> = None;
static mut SWARM: Option<Swarm<DecentNetworkBehaviour>> = None;
static mut EVENT_CHANNEL: Option<(
    UnboundedSender<SwarmEventExt>,
    UnboundedReceiver<SwarmEventExt>,
)> = None;

#[derive(Clone)]
pub struct NetConfig {
    pub data_dir: String,
    pub server_mode: bool,
    pub debug_mode: bool,
    pub boot_nodes_file: String,
    pub default_port: u32,
    pub ping_interval: u32,
    pub enable_identify: bool,
    pub local_discovery: bool,
    pub live_connection: bool,
    pub relay_dial_mode: bool,
}

fn version() -> String {
    "0.1.0".to_string()
}

fn init_runtime() -> Result<bool, &'static str> {
    let runtime = Runtime::new().map_err(|_| "Failed to initialize tokio runtime")?;
    let result;
    unsafe {
        RUNTIME = Some(runtime);
        result = RUNTIME.is_some();
    }
    Ok(result)
}

fn initialise_default() -> Result<String, &'static str> {
    init_runtime()?;
    let private_key = SecretKey::generate();
    let net_cfg = NetworkConfig {
        boot_nodes: load_bootnodes("boot_nodes.txt"),
        ..Default::default()
    };
    initialise_with_private_key(net_cfg, private_key)
}

pub fn initialise_with_network(
    network_config: NetworkConfig,
    network: Network,
) -> Result<String, &'static str> {
    init_runtime()?;
    let swarm = super::lib::initialise_with_network(network_config.clone(), network.clone())?;
    unsafe {
        NETWORK_CONFIG = Some(network_config);
        SWARM = Some(swarm);
        EVENT_CHANNEL = Some(unbounded());
        NETWORK = Some(network.clone());
    }
    Ok(network.public_key().to_peer_id().to_string())
}

pub fn initialise_with_private_key(
    network_config: NetworkConfig,
    private_key: SecretKey,
) -> Result<String, &'static str> {
    let network = unsafe {
        let bytes: Vec<u8> = private_key.as_ref().to_vec();
        let network = Network::from_bytes(bytes);
        NETWORK = Some(network.clone());
        network
    };
    initialise_with_network(network_config, network)
}

pub fn start_network() -> Result<String, String> {
    unsafe {
        if SWARM.is_none()
            || NETWORK.is_none()
            || NETWORK_CONFIG.is_none()
            || EVENT_CHANNEL.is_none()
        {
            return Err("Initialise Network before invoking start_network()".to_string());
        }
        let swarm = SWARM.as_mut().unwrap();
        let config = NETWORK_CONFIG.as_mut().unwrap();
        if RUNTIME.is_none() {
            Err("Initialise Runtime before invoking start_network()".to_string())
        } else {
            let runtime = RUNTIME.as_mut().unwrap();
            let result = runtime.block_on(async { Network::start_listening(swarm, config) });
            //TODO: Handle All possible errors
            if let (Err(error), Err(_error2)) = result {
                Err(error.to_string())
            } else {
                Ok(format!(
                    "Ipv4 : {:?}, Ipv6 : {:?}",
                    result.0.unwrap(),
                    result.1.unwrap()
                ))
            }
        }
    }
}

pub fn consume_swarm() -> Swarm<DecentNetworkBehaviour> {
    unsafe {
        if SWARM.is_none() {
            panic!("Initialise Network before invoking consume_swarm()");
        }
        SWARM.take().unwrap()
    }
}

fn list_nodes(with_addresses: bool) -> Vec<String> {
    let swarm = unsafe { SWARM.as_mut().unwrap() };
    let result = unsafe {
        RUNTIME.as_mut().unwrap().block_on(async {
            DecentNetworkBehaviour::list_nodes(swarm)
                .into_iter()
                .map(|node| {
                    let addrs = if with_addresses {
                        node.multiaddr
                            .iter()
                            .fold(None, |prev: Option<Vec<String>>, curr| {
                                if let Some(mut value) = prev {
                                    value.push(curr.to_string());
                                    Some(value)
                                } else {
                                    Some(vec![curr.clone()])
                                }
                            })
                    } else {
                        None
                    };
                    NetworkNode {
                        network_id: node.network_id,
                        multiaddr: if let Some(addrs) = addrs {
                            addrs
                        } else {
                            vec![]
                        },
                    }
                })
                .collect::<Vec<NetworkNode>>()
        })
    };
    result
        .into_iter()
        .map(|node| {
            let mut result = node.network_id.to_string();
            if !node.multiaddr.is_empty() {
                result.push(' ');
                result.push_str(&node.multiaddr.join("::"));
            }
            result
        })
        .collect()
}

fn list_listeners() -> Vec<String> {
    let swarm = unsafe { SWARM.as_mut().unwrap() };
    let result = unsafe {
        RUNTIME.as_mut().unwrap().block_on(async {
            swarm
                .listeners()
                .into_iter()
                .map(|node| node.to_string())
                .collect::<Vec<_>>()
        })
    };
    result
}

fn list_bootnodes(with_addresses: bool) -> Vec<String> {
    let config = unsafe { NETWORK_CONFIG.as_mut().unwrap() };
    let result = unsafe {
        RUNTIME.as_mut().unwrap().block_on(async {
            config
                .boot_nodes
                .clone()
                .into_iter()
                .map(|node| {
                    let mut result = node.network_id.to_string();
                    if with_addresses && !node.multiaddr.is_empty() {
                        result.push(' ');
                        result.push_str(&node.multiaddr.to_string());
                    }
                    result
                })
                .collect::<Vec<_>>()
        })
    };
    result
}

fn list_external_addrs() -> Vec<String> {
    let swarm = unsafe { SWARM.as_mut().unwrap() };
    let result = unsafe {
        RUNTIME.as_mut().unwrap().block_on(async {
            swarm
                .external_addresses()
                .into_iter()
                .map(|node| node.addr.to_string())
                .collect::<Vec<_>>()
        })
    };
    result
}

fn ping(network_id: String) -> Result<bool, String> {
    let swarm = unsafe { SWARM.as_mut().unwrap() };
    let peer_id = PeerId::from_str(&network_id);
    if let Err(err) = peer_id {
        return Err(err.to_string());
    }
    unsafe {
        RUNTIME.as_mut().unwrap().block_on(async {
            DecentNetworkBehaviour::ping(swarm.behaviour_mut(), &peer_id.unwrap())
        })
    };
    Ok(true)
}

fn dial(address: String) -> Result<bool, String> {
    let swarm = unsafe { SWARM.as_mut().unwrap() };
    let runtime = unsafe { RUNTIME.as_mut() };
    if runtime.is_none() {
        return Err("Initialise Runtime before invoking dial()".to_string());
    }
    let result = runtime
        .unwrap()
        .block_on(async { DecentNetworkBehaviour::dial_addr(swarm, &address) });
    if let Err(err) = result {
        Err(err)
    } else {
        Ok(true)
    }
}

fn get_nodes(network_id: String) -> Result<bool, String> {
    let swarm = unsafe { SWARM.as_mut().unwrap() };
    let network_id = PeerId::from_str(&network_id);
    if let Err(err) = network_id {
        return Err(err.to_string());
    }
    let runtime = unsafe { RUNTIME.as_mut() };
    if runtime.is_none() {
        return Err("Initialise Runtime before invoking dial()".to_string());
    }
    runtime
        .unwrap()
        .block_on(async { DecentNetworkBehaviour::get_nodes(swarm, &network_id.unwrap()) });
    Ok(true)
}

fn get_nodes_from_bootnodes() -> Result<bool, String> {
    let swarm = unsafe { SWARM.as_mut().unwrap() };
    let runtime = unsafe { RUNTIME.as_mut() };
    if runtime.is_none() {
        return Err("Initialise Runtime before invoking dial()".to_string());
    }
    runtime
        .unwrap()
        .block_on(async { DecentNetworkBehaviour::get_nodes_from_bootnodes(swarm) });
    Ok(true)
}

impl From<NetConfig> for NetworkConfig {
    fn from(cfg: NetConfig) -> Self {
        let default_port = cfg.default_port as u16;
        let ping_interval = Duration::from_millis(cfg.ping_interval as u64);
        NetworkConfig {
            server_mode: cfg.server_mode,
            debug_mode: cfg.debug_mode,
            enable_identify: cfg.enable_identify,
            live_connection: cfg.live_connection,
            local_discovery: cfg.local_discovery,
            relay_dial_mode: cfg.relay_dial_mode,
            default_port,
            ping_interval,
            ..Default::default()
        }
    }
}

impl Default for NetConfig {
    fn default() -> Self {
        Self {
            data_dir: "data".to_string(),
            server_mode: false,
            debug_mode: false,
            boot_nodes_file: "boot_nodes.txt".to_string(),
            default_port: 0,
            ping_interval: 0,
            enable_identify: false,
            local_discovery: false,
            live_connection: false,
            relay_dial_mode: false,
        }
    }
}

pub struct SwarmEventExt(String);

impl ToString for SwarmEventExt {
    fn to_string(&self) -> String {
        format!("{:?}", self.0)
    }
}

impl Stream for SwarmEventExt {
    type Item = SwarmEventExt;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let runtime = unsafe { RUNTIME.as_mut().unwrap() };
        let swarm = unsafe { SWARM.as_mut().unwrap() };
        let event = runtime.block_on(swarm.select_next_some());
        // let res = Some(SwarmEventExt(Some(format!("{:?}", event))));
        // match res {
        //     Some(event) => Poll::Ready(Some(event)),
        //     None => Poll::Pending,
        // }
        Poll::Ready(Some(SwarmEventExt(format!("{:?}", event))))
    }
}

impl FusedStream for SwarmEventExt {
    fn is_terminated(&self) -> bool {
        false
    }
}

pub struct SwarmExt();

impl SwarmExt {
    pub fn subscribe_events(&self) -> impl Stream<Item = SwarmEventExt> {
        SwarmEventExt("Started".to_string())
    }
}

pub fn swarm() -> SwarmExt {
    SwarmExt()
}

#[cfg(test)]
mod tests {
    use crate::api::build_development_swarm1;
    use libp2p::futures::StreamExt;

    #[tokio::test]
    async fn test_swarm_stream() {
        let swarm = super::SwarmExt();
        let mut sub = swarm.subscribe_events();
        let _swarm = build_development_swarm1().await;
        while let Some(event) = sub.next().await {
            println!("{}", event.to_string());
        }
    }
}
