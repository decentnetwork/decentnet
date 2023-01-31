use std::path::Path;

use clap::{Arg, ArgMatches, Command};
use lazy_static::lazy_static;
use libp2p::floodsub::Topic;
use log::info;

use decentnet::{
    api::lib::{ipv6_supported, load_bootnodes},
    network::{identity::Network, BootNode, IdentityImpl, NetworkId},
};

const DATA_DIR: &str = "data/";

lazy_static! {
    pub static ref ARGS: ArgMatches = {
        Command::new("decentnet")
            .version("0.1")
            .author("Pramukesh")
            .about("A Modern Decentralised Network Protocol")
            .arg(
                Arg::new("new_node")
                    .short('n')
                    .long("new_node")
                    .help("Use New Node Id, Instead of Existing")
                    .required(false),
            )
            .arg(
                Arg::new("file_log")
                    .short('f')
                    .long("file_log")
                    .help("Use New Node Id, Instead of Existing")
                    .takes_value(true)
                    .default_value("true")
                    .required(false),
            )
            .arg(
                Arg::new("custom_port")
                    .short('c')
                    .long("custom_port")
                    .help("Use Custom Port, Instead of Default Port: 21697")
                    .takes_value(true)
                    .default_value("21697")
                    .required(false),
            )
            .arg(
                Arg::new("ui_port")
                    .short('u')
                    .long("ui_port")
                    .help("Use UI Custom Port, Instead of Default Port: 12721")
                    .takes_value(true)
                    .default_value("12721")
                    .required(false),
            )
            .arg(
                Arg::new("listen_mode")
                    .short('r')
                    .long("listen_mode")
                    .help("Wheather to start client in listen mode")
                    .required(false),
            )
            .arg(
                Arg::new("server_mode")
                    .short('s')
                    .long("server")
                    .help("Run App in Server Mode.")
                    .required(false),
            )
            .arg(
                Arg::new("boot_nodes")
                    .short('b')
                    .long("boot_nodes")
                    .takes_value(true)
                    .default_value("boot_nodes.txt")
                    .help("BootNodes Files that contains PeedId and Address")
                    .required(false),
            )
            .arg(
                Arg::new("ping_interval")
                    .short('p')
                    .long("ping_interval")
                    .takes_value(true)
                    .default_value("300")
                    .help("Defualt Ping Time between Nodes")
                    .required(false),
            )
            .arg(
                Arg::new("local_discovery")
                    .short('d')
                    .long("local_discovery")
                    .takes_value(true)
                    .default_value("false")
                    .help("Enable Discovery of Node in Local Network")
                    .required(false),
            )
            .arg(
                Arg::new("enable_identify")
                    .short('i')
                    .long("enable_identify")
                    .takes_value(false)
                    .help("Enable Identify Protocol of Node in Local Network")
                    .required(false),
            )
            .arg(
                Arg::new("live_connection")
                    .short('l')
                    .long("live_connection")
                    .takes_value(true)
                    .default_value("true")
                    .help("Whether Ping Connection should is to be live")
                    .required(false),
            )
            .arg(
                Arg::new("use_yamux")
                    .short('y')
                    .long("use_yamux")
                    .takes_value(true)
                    .default_value("false")
                    .help("Whether Yamux should be used as default multiplexer")
                    .required(false),
            )
            .get_matches()
    };
    pub static ref ANNONYMOUS_USER: bool = ARGS.occurrences_of("new_node") > 0;
    pub static ref RELAY_LISTEN_MODE: bool = ARGS.occurrences_of("listen_mode") > 0;
    pub static ref SERVER_MODE: bool = ARGS.occurrences_of("server_mode") > 0;
    pub static ref ENABLE_IDENTIFY: bool = ARGS.occurrences_of("enable_identify") > 0;
    pub static ref FILE_LOG_MODE: bool = ARGS.value_of("file_log").unwrap().parse().unwrap();
    pub static ref LOCAL_DISCOVERY: bool = ARGS.value_of("local_discovery").unwrap().parse().unwrap();
    pub static ref USE_YAMUX: bool = ARGS.value_of("use_yamux").unwrap().parse().unwrap();
    pub static ref IPV6_SUPPORTED: bool = ipv6_supported();
    pub static ref DEFAULT_PORT: u16 = if *SERVER_MODE {
        26117
    } else {
        ARGS.value_of("custom_port").unwrap().parse().unwrap()
    };
    pub static ref UI_PORT: u16 =
        ARGS.value_of("ui_port").unwrap().parse().unwrap();
    pub static ref USER_FILE: String = DATA_DIR.to_owned() + "usercredentials";
    pub static ref USER_EXISTS: bool = Path::new(&*USER_FILE).exists();
    pub static ref NETWORK: Network = {
        if !*ANNONYMOUS_USER && *USER_EXISTS {
            let bytes = std::fs::read(&*USER_FILE).unwrap();
            Network::from_bytes(bytes)
        } else {
            let (private_key, network) = Network::gen_random_id_with_private();
            let path = Path::new(DATA_DIR);
            if !path.exists() {
                std::fs::create_dir_all(path).unwrap();
            }
            if !*ANNONYMOUS_USER {
                let mut file = std::fs::File::create(&*USER_FILE).unwrap();
                std::io::Write::write_all(&mut file, private_key.as_ref()).unwrap();
            } else {
                //? Show this when you are in persistent mode for annonymous users.
                // info!(
                //     "Please Save your Private Key to Access Your Data, \nPrivate Key : {:?} ",
                //     keypair.secret().as_ref().as_ref()
                // );
                info!("You're an Annonymous User, Your Data will be lost on Exit");
            }
            network
        }
    };
    pub static ref NETWORK_ID: NetworkId = NetworkId::from_public_key(&NETWORK.public_key());
    pub static ref BOOT_NODES: Vec<BootNode> = {
        if !*SERVER_MODE {
            load_bootnodes(ARGS.value_of("boot_nodes").unwrap())
        } else {
            vec![]
        }
    };
    pub static ref PING_INTERVAL: u64 = ARGS.value_of("ping_interval").unwrap().parse::<u64>().unwrap();
    pub static ref LIVE_CONNECTION: bool = if *SERVER_MODE {
            true
        } else {
            ARGS.value_of("live_connection").unwrap().parse::<bool>().unwrap()
        };
    pub static ref TOPIC: Topic = Topic::new("recipes");
    pub static ref STORAGE_DIR_PATH: String = {
        let network_id = &*NETWORK_ID.to_base58();
        format!("data/{}", network_id,)
    };
    pub static ref STORAGE_FILE_PATH: String = format!("{}/{}", &*STORAGE_DIR_PATH, "recipes.json");
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_ipv6_supported() {
        println!("Is Ipv6 Supported : {}", ipv6_supported());
    }
}
