use crate::network::{
    behaviour::DecentNetworkBehaviour, config::NetworkConfig, identity::Network,
    protocol::DecentNetRequest, BootNode, IdentityImpl, NetworkNode,
};
use libp2p::{
    swarm::{behaviour::toggle::Toggle, AddressRecord},
    Multiaddr, PeerId, Swarm,
};
use log::info;

#[allow(unused)]
pub mod common;
pub mod ffi;
pub mod lib;

impl DecentNetworkBehaviour {
    pub fn list_nodes(swarm: &mut Swarm<Self>) -> Vec<NetworkNode> {
        info!("Discovered Nodes:");
        let config = swarm.behaviour_mut().config.clone();
        let mut nodes = vec![];
        swarm
            .behaviour_mut()
            .kademlia
            .kbuckets()
            .for_each(|bucketref| {
                let filtered = bucketref
                    .iter()
                    .filter(|refview| {
                        let network_id = *refview.node.key.preimage();
                        let bootnode = config.boot_nodes.first();
                        if let Some(bootnode) = bootnode {
                            network_id != bootnode.network_id
                        } else {
                            true
                        }
                    })
                    .map(|refview| {
                        let network_id = *refview.node.key.preimage();
                        let multiaddr = (refview.node.value)
                            .clone()
                            .into_vec()
                            .into_iter()
                            .map(|addr| addr.to_string())
                            .collect();
                        NetworkNode {
                            network_id: network_id.to_base58(),
                            multiaddr,
                        }
                    });
                nodes.extend(filtered);
            });
        nodes
    }

    pub fn get_nodes_from_bootnodes(swarm: &mut Swarm<Self>) {
        info!("Sending request to get known peers from boot node");
        swarm
            .behaviour()
            .config
            .boot_nodes
            .clone()
            .into_iter()
            .map(|bootnode| bootnode.network_id)
            .into_iter()
            .for_each(|network_id| {
                Self::get_nodes(swarm, &network_id);
            });
    }

    pub fn get_nodes(swarm: &mut Swarm<Self>, network_id: &PeerId) {
        info!("Sending request to {} to get known peers", network_id);
        swarm
            .behaviour_mut()
            .protocol
            .send_request(network_id, DecentNetRequest::GetNetworkNodes);
    }

    pub fn dial_addr(swarm: &mut Swarm<Self>, addr: &str) -> Result<(), String> {
        let result = addr.parse::<Multiaddr>();
        if let Err(err) = result {
            return Err(err.to_string());
        }
        info!("Dialing address {}", addr);
        let result = swarm.dial(result.unwrap());
        if let Err(e) = result {
            return Err(e.to_string());
        }
        Ok(())
    }

    pub fn ping(&mut self, network_id: &PeerId) {
        info!("Sending ping to {}", network_id);
        self.protocol
            .send_request(network_id, DecentNetRequest::Ping);
    }

    pub fn external_addresses(swarm: &mut Swarm<Self>) -> impl Iterator<Item = &AddressRecord> {
        swarm.external_addresses()
    }
}

pub async fn build_development_swarm1() -> Swarm<DecentNetworkBehaviour> {
    let config = development_net_conf(26117, true);
    build_development_swarm(&config).await
}

pub async fn build_development_swarm2() -> Swarm<DecentNetworkBehaviour> {
    let config = development_net_conf(26118, false);
    build_development_swarm(&config).await
}

pub async fn build_development_swarm(config: &NetworkConfig) -> Swarm<DecentNetworkBehaviour> {
    let (_, network) = Network::gen_random_id_with_private();
    let (client, transport) = network.build_transport(config.server_mode, config.use_yamux);
    let behaviour = network.build_behaviour(config.clone(), client).await;
    network.build_swarm(transport, behaviour)
}

pub fn development_net_conf(port: u16, dial_mode: bool) -> NetworkConfig {
    let boot_nodes = vec![BootNode {
        network_id: "12D3KooWQcxKU41WtSYMhsUFpizaejtm4wCHasAj7MTvZxVZeCh4"
            .parse()
            .unwrap(),
        multiaddr: "/ip4/138.68.180.101/tcp/26117".parse().unwrap(),
    }];
    NetworkConfig {
        boot_nodes: boot_nodes.clone(),
        default_port: port,
        live_connection: true,
        debug_mode: true,
        relay_id: Some(boot_nodes.first().unwrap().network_id.to_string()),
        relay_dial_mode: dial_mode,
        ..Default::default()
    }
}

pub fn build_development_swarm_sync() -> Swarm<DecentNetworkBehaviour> {
    let config = development_net_conf(26117, true);
    let (_, network) = Network::gen_random_id_with_private();
    let (client, transport) = network.build_transport(config.server_mode, config.use_yamux);
    let behaviour = network.build_behaviour_sync(config, client, Toggle::from(None));
    network.build_swarm(transport, behaviour)
}
