use decentnet::{
    api::common::{consume_swarm, initialise_with_network, start_network},
    network::{
        behaviour::DecentNetworkBehaviour,
        config::{NetworkConfig, RENDEZVOUS_NAMESPACE},
        identity::Network,
        protocol::DecentNetRequest,
        NetworkEvent, NetworkId, RendezvousBehaviour,
    },
};
use libp2p::{
    floodsub::FloodsubEvent,
    futures::StreamExt,
    rendezvous,
    swarm::{NetworkBehaviour, SwarmEvent},
    Multiaddr, PeerId, Swarm,
};
use log::{error, info, LevelFilter};
use log4rs::{
    append::file::FileAppender,
    config::{Appender, Root},
    encode::pattern::PatternEncoder,
    Config,
};

use std::{
    collections::HashSet,
    fs::{self, remove_dir_all, File, OpenOptions},
    io::{self, BufRead, BufReader},
    path::{self, Path},
    thread,
    time::{self, Duration},
};
use tokio::{runtime::Runtime, sync::mpsc};

mod constants;
mod handlers;
mod models;
mod ui;
mod utils;

use crate::{
    constants::{
        ANNONYMOUS_USER, BOOT_NODES, DEFAULT_PORT, ENABLE_IDENTIFY, FILE_LOG_MODE, LIVE_CONNECTION,
        LOCAL_DISCOVERY, NETWORK, NETWORK_ID, PING_INTERVAL, RELAY_LISTEN_MODE, SERVER_MODE,
        STORAGE_DIR_PATH, TOPIC, UI_PORT,
    },
    handlers::{
        handle_broadcast_recipe, handle_create_recipe, handle_delete_recipe, handle_list_nodes,
        handle_list_recipes, handle_publish_recipe, respond_with_public_recipes,
    },
    models::{EventType, ListBroadcast, ListMode, ListRequest, ListResponse},
    ui::WebServer,
    utils::create_default_recipes,
};

// #[tokio::main]
fn main() {
    if !*FILE_LOG_MODE || *SERVER_MODE {
        pretty_env_logger::init();
    } else {
        let log_path = "data/decentnet.log";
        if Path::new(log_path).exists() {
            fs::remove_file(Path::new(log_path)).unwrap();
        }
        let logfile = FileAppender::builder()
            .encoder(Box::new(PatternEncoder::new("{l} - {m}\n")))
            .build(log_path)
            .unwrap();
        let config_builder =
            Config::builder().appender(Appender::builder().build("logfile", Box::new(logfile)));
        let config = config_builder
            .build(
                Root::builder()
                    .appender("logfile")
                    .build(LevelFilter::Debug),
            )
            .unwrap();
        log4rs::init_config(config).unwrap();
    };
    info!("NETWORK Id: {}", NETWORK_ID.clone());
    // create_default_recipes().await;
    let server = WebServer::new("127.0.0.1".to_string(), *UI_PORT, NETWORK.clone());
    // let (response_sender, mut response_rcv) = mpsc::unbounded_channel();
    // let (broadcast_sender, mut broadcast_rcv) = mpsc::unbounded_channel();
    let network = NETWORK.clone();
    let mut network_config = NetworkConfig {
        server_mode: *SERVER_MODE,
        boot_nodes: (*BOOT_NODES).clone(),
        default_port: *DEFAULT_PORT,
        ping_interval: Duration::from_secs(*PING_INTERVAL),
        live_connection: *LIVE_CONNECTION,
        local_discovery: *LOCAL_DISCOVERY,
        enable_identify: *ENABLE_IDENTIFY,
        debug_mode: true,
        relay_id: if !*SERVER_MODE {
            Some(
                (*BOOT_NODES)
                    .clone()
                    .first()
                    .unwrap()
                    .network_id
                    .to_string(),
            )
        } else {
            None
        },
        relay_dial_mode: !*RELAY_LISTEN_MODE,
        ..Default::default()
    };
    // let (client, transport) = network.build_transport(*SERVER_MODE);
    // let behaviour = network
    //     .build_behaviour(network_config.clone(), client)
    //     .await;
    let result = initialise_with_network(network_config.clone(), network.clone());

    let _peer_id = result.unwrap();

    let result = start_network();
    if let Err(error) = &result {
        error!("Transport Errror : {:?}", error);
    }
    let _listener_id = result.unwrap();
    let mut swarm = consume_swarm();

    // let mut discover_tick = time::interval(Duration::from_secs(30));
    // let mut ping_tick = time::interval(network_config.ping_interval);
    // let mut stdin = io::BufReader::new(io::stdin()).lines();
    // let mut last_known_peers = HashSet::new();

    swarm.behaviour_mut().floodsub.subscribe(TOPIC.clone());
    if !*SERVER_MODE {
        // load_nodes_file(&mut swarm).await;
    }
    let exit: bool;
    let mut dialed_boot_nodes = false;

    loop {
        let evt = {
            Runtime::new().unwrap().block_on(async {
                if !dialed_boot_nodes {
                    if !*SERVER_MODE {
                        swarm
                            .dial((*BOOT_NODES).first().unwrap().multiaddr.clone())
                            .unwrap();
                    }
                    dialed_boot_nodes = true;
                }
                let event = swarm.select_next_some().await;
                println!("{:?}", event);
                match event {
                    SwarmEvent::Behaviour(network_event) => {
                        println!("{:?}", network_event);
                        DecentNetworkBehaviour::handle_network_event(
                            network_event,
                            &mut network_config,
                            &mut swarm,
                        );
                    }
                    event => {
                        println!("{:?}", event);
                        DecentNetworkBehaviour::handle_swarm_event(
                            &mut swarm,
                            event,
                            &network_config,
                        );
                    }
                }
            });
            None
        };
        if let Some(event) = evt {
            match event {
                EventType::Response(resp) => {
                    let json = serde_json::to_string(&resp).expect("can jsonify response");
                    swarm
                        .behaviour_mut()
                        .floodsub
                        .publish(TOPIC.clone(), json.as_bytes());
                }
                EventType::Broadcast(broadcast) => {
                    let json = serde_json::to_string(&broadcast).expect("can jsonify response");
                    swarm
                        .behaviour_mut()
                        .floodsub
                        .publish(TOPIC.clone(), json.as_bytes());
                }
                EventType::Input(line) => match line.as_str() {
                    cmd if cmd.starts_with('z') => {
                        exit = true;
                        break;
                    }
                    cmd if cmd.starts_with("ls p") => {
                        let rem = cmd.strip_prefix("ls p ");
                        match rem {
                            Some(_) => handle_list_nodes(&mut swarm, true),
                            None => handle_list_nodes(&mut swarm, false),
                        }
                    }
                    cmd if cmd.starts_with("ls b") => {
                        let rem = cmd.strip_prefix("ls b ");
                        match rem {
                            Some(_) => handle_list_nodes(&mut swarm, true),
                            None => handle_list_nodes(&mut swarm, false),
                        }
                    }
                    cmd if cmd.starts_with("ls l") => {
                        swarm
                            .listeners()
                            .into_iter()
                            .for_each(|addr| info!("{}", addr));
                    }
                    cmd if cmd.starts_with("ls e") => {
                        info!(
                            "External Address : {:?}",
                            DecentNetworkBehaviour::external_addresses(&mut swarm)
                                .into_iter()
                                .collect::<Vec<_>>()
                        );
                    }
                    cmd if cmd.starts_with("ping ") => {
                        let rem = cmd.strip_prefix("ping ");
                        match rem {
                            Some(network_id) => {
                                let network_id = network_id.parse::<PeerId>().unwrap();
                                DecentNetworkBehaviour::ping(swarm.behaviour_mut(), &network_id);
                            }
                            None => error!("Please provide a network id"),
                        }
                    }
                    cmd if cmd.starts_with("d ") => {
                        let rem = cmd.strip_prefix("d ");
                        match rem {
                            Some(addr) => {
                                let result = DecentNetworkBehaviour::dial_addr(&mut swarm, addr);
                                match result {
                                    Ok(()) => info!("Dialed {:?}", addr),
                                    Err(e) => {
                                        error!("Failed to dial {:?} : {:?}", addr, e)
                                    }
                                }
                            }
                            None => error!("Please provide an addr"),
                        }
                    }
                    cmd if cmd.starts_with("get peers ") => {
                        let rem = cmd.strip_prefix("get peers ");
                        match rem {
                            Some(network_id) => {
                                if network_id == "b" {
                                    DecentNetworkBehaviour::get_nodes_from_bootnodes(&mut swarm);
                                } else {
                                    let network_id = network_id.parse::<PeerId>();
                                    if let Ok(network_id) = network_id {
                                        DecentNetworkBehaviour::get_nodes(&mut swarm, &network_id);
                                    } else {
                                        error!("Please provide a valid network id")
                                    }
                                }
                            }
                            None => error!("Please provide a network id"),
                        }
                    }
                    // "b r" | "broadcast r" => {
                    //     handle_broadcast_recipe(&mut swarm, broadcast_sender.clone()).await
                    // }
                    // cmd if cmd.starts_with("ls r") => handle_list_recipes(cmd, &mut swarm).await,
                    // cmd if cmd.starts_with("create r") | cmd.starts_with("c r") => {
                    //     handle_create_recipe(cmd).await
                    // }
                    // cmd if cmd.starts_with("delete r") | cmd.starts_with("d r") => {
                    //     handle_delete_recipe(cmd).await
                    // }
                    // cmd if cmd.starts_with("publish r") | cmd.starts_with("p r") => {
                    //     handle_publish_recipe(cmd).await
                    // }
                    _ => error!("unknown command"),
                },
            }
        }
    }

    let unique_nodes = network.get_known_nodes(&mut swarm);
    if !unique_nodes.is_empty() {
        let mut file: File = {
            let path = Path::new("data/nodes");
            if path.exists() {
                OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .open(path)
                    .expect("can open file")
            } else {
                File::create(path).expect("can create file")
            }
        };
        let mut nodes_string = "".to_string();
        for node in &unique_nodes {
            nodes_string.push_str(&node.0.to_base58());
            nodes_string.push(' ');
            let mut addrs_str: String = String::new();
            for addr in &node.1 {
                if node.1.last().unwrap() != addr {
                    addrs_str.push_str(&format!("{}::", addr));
                } else {
                    addrs_str.push_str(&addr.to_string());
                }
            }
            nodes_string.push_str(&addrs_str);
            nodes_string.push('\n');
        }
        //TODO! Use Blockchain Style here and Encryption to protect other users privacy.

        let result = File::write_all(&mut file, nodes_string.as_bytes());
        if result.is_err() {
            error!("Failed to write Nodes to file");
        } else {
            info!("Wrote {} Nodes to file", &unique_nodes.len());
        }
    }
    if exit {
        info!("Exiting...");
        swarm.fuse().next();
    }
    if *ANNONYMOUS_USER {
        remove_dir_all(path::Path::new(&*STORAGE_DIR_PATH)).expect("can remove file");
    }
}

// async fn user_input(stdin: &mut Lines<BufReader<Stdin>>) -> Option<String> {
//     // if *SERVER_MODE {
//     //     None
//     // } else {
//     Some(stdin.next_line().await.unwrap().unwrap())
//     // }
// }
use std::io::Write;

async fn load_nodes_file(swarm: &mut Swarm<DecentNetworkBehaviour>) {
    let path = Path::new("data/nodes");
    if !path.exists() {
        File::create(path).expect("can create file");
    }
    let file = File::open("data/nodes").unwrap();
    let mut file = BufReader::new(file);
    let mut content = String::new();
    file.read_line(&mut content).unwrap();
    let mut nodes = vec![];
    for line in content.split('\n') {
        if line.is_empty() {
            continue;
        }
        let (network_id_str, addresses) = line.split_once(' ').unwrap();
        let network_id: NetworkId = network_id_str.parse().unwrap();
        let mut address_vec = vec![];
        for address in addresses.split("::") {
            address_vec.push(address.parse::<Multiaddr>().unwrap());
        }
        nodes.push((network_id, address_vec));
    }
    Network::load_nodes(swarm, nodes);
}
