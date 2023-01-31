use libp2p::{
    core::{
        connection::ListenerId,
        muxing::StreamMuxerBox,
        transport::{Boxed, OrTransport},
        upgrade::Version,
    },
    dcutr::behaviour::Behaviour as DCUtR,
    floodsub::{Floodsub, FloodsubEvent},
    identify::{Identify, IdentifyConfig},
    kad::{store::MemoryStore, Kademlia},
    mdns::Mdns,
    mplex::MplexConfig,
    multiaddr::Protocol,
    noise::NoiseConfig,
    ping::{Ping, PingConfig},
    relay::v2::{
        client::Client as RelayClient,
        relay::{Config as RelayConfig, Relay},
    },
    request_response::{ProtocolSupport, RequestId, RequestResponse, RequestResponseConfig},
    swarm::{
        behaviour::toggle::Toggle, NetworkBehaviour, NetworkBehaviourEventProcess, SwarmBuilder,
    },
    tcp::TokioTcpConfig,
    yamux::YamuxConfig,
    Multiaddr, NetworkBehaviour, Swarm, Transport, TransportError,
};
use log::info;
use std::{
    collections::HashSet,
    io, iter,
    net::{Ipv4Addr, Ipv6Addr},
    time::Duration,
};

use crate::network::{
    config::{NetworkConfig, IDENTIFY_PROTOCOL_VERSION},
    identity::{Network, RendezvousBehaviourImpl},
    utils::get_external_addrs,
    IdentityImpl, NetworkEvent, NetworkId, RendezvousBehaviour,
};

use super::protocol::{DecentNetCodec, DecentNetProtocol};

#[derive(NetworkBehaviour)]
#[behaviour(event_process = false)]
#[behaviour(out_event = "NetworkEvent")]
pub struct DecentNetworkBehaviour {
    pub ping: Ping,
    pub floodsub: Floodsub,
    pub kademlia: Kademlia<MemoryStore>,
    pub mdns: Toggle<Mdns>,
    pub identify: Identify,
    pub protocol: RequestResponse<DecentNetCodec>,
    pub relay_server: Toggle<Relay>,
    pub relay_client: Toggle<RelayClient>,
    pub dcutr: Toggle<DCUtR>,
    #[behaviour(ignore)]
    pub rendezvous: RendezvousBehaviour,
    #[behaviour(ignore)]
    pub config: NetworkConfig,
    #[behaviour(ignore)]
    pub requests: HashSet<(RequestId, NetworkId, bool)>,
}

impl AsMut<DecentNetworkBehaviour> for DecentNetworkBehaviour {
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}

impl NetworkBehaviourEventProcess<FloodsubEvent> for DecentNetworkBehaviour {
    fn inject_event(&mut self, event: FloodsubEvent) {
        info!("Unhandled {:?}", event);
    }
}

impl Network {
    pub async fn build_behaviour(
        &self,
        config: NetworkConfig,
        client: Option<RelayClient>,
    ) -> DecentNetworkBehaviour {
        let mdns = if !config.server_mode && config.local_discovery {
            let mdns = Mdns::new(Default::default()).await;
            let mdns = if let Ok(mdns) = mdns {
                Some(mdns)
            } else {
                None
            };
            Toggle::from(mdns)
        } else {
            Toggle::from(None)
        };
        self.build_behaviour_sync(config, client, mdns)
    }

    pub fn build_behaviour_sync(
        &self,
        config: NetworkConfig,
        client: Option<RelayClient>,
        mdns: Toggle<Mdns>,
    ) -> DecentNetworkBehaviour {
        DecentNetworkBehaviour {
            ping: Ping::new(PingConfig::new()),
            floodsub: Floodsub::new(self.id()),
            kademlia: {
                let store = MemoryStore::new(self.id());
                Kademlia::new(self.id(), store)
            },
            mdns,
            identify: Identify::new(IdentifyConfig::new(
                IDENTIFY_PROTOCOL_VERSION.to_string(),
                self.public_key(),
            )),
            relay_server: {
                if config.server_mode {
                    let config = RelayConfig {
                        max_reservations: 1024,
                        max_circuits: 1024,
                        max_circuits_per_peer: 8,
                        max_circuit_duration: Duration::from_secs(30 * 60),
                        max_circuit_bytes: (1 << 17) * 8,
                        ..Default::default()
                    };
                    Toggle::from(Some(Relay::new(self.id(), config)))
                } else {
                    Toggle::from(None)
                }
            },
            relay_client: {
                if !config.server_mode {
                    Toggle::from(client)
                } else {
                    Toggle::from(None)
                }
            },
            dcutr: if !config.server_mode {
                Toggle::from(Some(DCUtR::new()))
            } else {
                Toggle::from(None)
            },
            protocol: {
                let secs = if config.live_connection { 60 * 10 } else { 10 };
                let duration = Duration::from_secs(secs);
                let cfg = RequestResponseConfig::default()
                    .set_connection_keep_alive(duration)
                    .set_request_timeout(Duration::from_secs(30))
                    .to_owned();
                let protocols = iter::once((DecentNetProtocol(), ProtocolSupport::Full));
                RequestResponse::new(DecentNetCodec(), protocols, cfg)
            },
            rendezvous: self.rendezvous_behaviour(config.server_mode),
            config,
            requests: HashSet::new(),
        }
    }

    pub fn build_transport(
        &self,
        server_mode: bool,
        use_yamux: bool,
    ) -> (Option<RelayClient>, Boxed<(NetworkId, StreamMuxerBox)>) {
        let auth_keys = self.auth_key_pair().expect("can create auth keys");
        let transp = TokioTcpConfig::new();
        if !server_mode {
            let transp = transp.port_reuse(true).nodelay(true);
            let (relay_transport, c) = RelayClient::new_transport_and_behaviour(self.id());
            let transport = OrTransport::new(relay_transport, transp)
                .upgrade(Version::V1)
                .authenticate(NoiseConfig::xx(auth_keys).into_authenticated());
            let transport = if use_yamux {
                transport.multiplex(YamuxConfig::default()).boxed()
            } else {
                transport.multiplex(MplexConfig::default()).boxed()
            };
            (Some(c), transport)
        } else {
            let transp = transp
                .upgrade(Version::V1)
                .authenticate(NoiseConfig::xx(auth_keys).into_authenticated());
            let transport = if use_yamux {
                transp.multiplex(YamuxConfig::default()).boxed()
            } else {
                transp.multiplex(MplexConfig::default()).boxed()
            };
            (None, transport)
        }
    }

    pub fn build_swarm<TBehaviour: NetworkBehaviour>(
        &self,
        transport: Boxed<(NetworkId, StreamMuxerBox)>,
        behaviour: TBehaviour,
    ) -> Swarm<TBehaviour> {
        SwarmBuilder::new(transport, behaviour, self.id())
            .dial_concurrency_factor(10_u8.try_into().unwrap())
            .executor(Box::new(|fut| {
                tokio::spawn(fut);
            }))
            .build()
    }

    pub fn start_listening<TBehaviour: NetworkBehaviour>(
        swarm: &mut Swarm<TBehaviour>,
        config: &NetworkConfig,
    ) -> (
        Result<ListenerId, TransportError<io::Error>>,
        Result<ListenerId, TransportError<io::Error>>,
    ) {
        let ipv4_addr = Multiaddr::empty()
            .with(Protocol::Ip4(Ipv4Addr::UNSPECIFIED))
            .with(Protocol::Tcp(config.default_port));
        let ipv6_addr = Multiaddr::empty()
            .with(Protocol::from(Ipv6Addr::UNSPECIFIED))
            .with(Protocol::Tcp(config.default_port));
        if !config.server_mode && !config.relay_dial_mode {
            let relayed_addr = Self::get_relayed_listening_addr(config);
            info!("Listening on relay address: {}", relayed_addr);
            Swarm::listen_on(swarm, relayed_addr).expect("can listen on relayed addr");
        };
        let ipv4_listner = Swarm::listen_on(swarm, ipv4_addr);
        let ipv6_listner = Swarm::listen_on(swarm, ipv6_addr);

        (ipv4_listner, ipv6_listner)
    }

    pub fn get_known_nodes(
        &self,
        swarm: &mut Swarm<DecentNetworkBehaviour>,
    ) -> Vec<(NetworkId, Vec<Multiaddr>)> {
        let boot_node_ids = swarm
            .behaviour_mut()
            .config
            .boot_nodes
            .clone()
            .iter()
            .map(|boot_node| boot_node.network_id)
            .collect::<Vec<_>>();
        let mut unique_nodes = vec![];
        let kad = &mut swarm.behaviour_mut().kademlia;
        let nodes = kad.kbuckets();
        nodes.for_each(|bucketref| {
            bucketref.iter().for_each(|refview| {
                let network_id = refview.node.key.preimage();
                let addr = &(refview.node.value).clone().into_vec();
                let addr = get_external_addrs(addr);
                if !boot_node_ids.contains(network_id) {
                    unique_nodes.push((*network_id, addr));
                }
            });
        });
        unique_nodes
    }

    pub fn load_nodes(
        swarm: &mut Swarm<DecentNetworkBehaviour>,
        nodes: Vec<(NetworkId, Vec<Multiaddr>)>,
    ) {
        for node in nodes {
            for addr in node.1 {
                swarm
                    .behaviour_mut()
                    .kademlia
                    .add_address(&node.0, addr.clone());
                swarm.dial(addr).unwrap();
            }
        }
    }

    pub fn get_relayed_listening_addr(config: &NetworkConfig) -> Multiaddr {
        let relay_node = config.boot_nodes.first().unwrap().clone();
        let relayed_addr = relay_node
            .multiaddr
            .with(Protocol::P2p(relay_node.network_id.into()))
            .with(Protocol::P2pCircuit);
        relayed_addr
    }
}

#[cfg(test)]
mod tests {
    use crate::network::{config::NetworkConfig, identity::Network, BootNode};

    #[test]
    fn test_relayed_listening_addr() {
        let addr_str = "/ip4/127.0.0.1/tcp/26117";
        let relay_id = "12D3KooWPjceQrSwdWXPyLLeABRXmuqt69Rg3sBYbU1Nft9HyQ6X";
        let net_con = NetworkConfig {
            boot_nodes: vec![BootNode {
                network_id: relay_id.parse().unwrap(),
                multiaddr: addr_str.parse().unwrap(),
            }],
            ..Default::default()
        };
        let addr = Network::get_relayed_listening_addr(&net_con).to_string();
        assert_eq!(addr, format!("{}/p2p/{}/p2p-circuit", addr_str, relay_id));
    }
}
