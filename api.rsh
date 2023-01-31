fn version() -> string;

fn init_runtime() -> Result<bool>;

fn initialise_default() -> Result<string>;

fn start_network() -> Result<string>;

fn list_nodes(with_addresses: bool) -> Iterator<string>;

fn list_listeners() -> Iterator<string>;

fn list_bootnodes(with_addresses: bool) -> Iterator<string>;

fn list_external_addrs() -> Iterator<string>;

fn ping(network_id: string) -> Result<bool>;

fn dial(address: string) -> Result<bool>;

fn get_nodes(network_id: string) -> Result<bool>;

fn get_nodes_from_bootnodes() -> Result<bool>;

object SwarmEventExt {
    fn to_string() -> string;
}

object SwarmExt {
    fn subscribe_events() -> Stream<SwarmEventExt>;
}

fn swarm() -> SwarmExt;