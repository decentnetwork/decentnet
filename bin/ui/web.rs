use decentnet::api::lib::{
    bootnodes, dial_addr, external_addresses, get_nodes, get_nodes_from_bootnodes, list_nodes,
    listeners, ping,
};
use libp2p::futures::channel::mpsc::unbounded;
use libp2p::futures::stream::SplitSink;
use libp2p::futures::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::fs;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::WebSocketStream;

use decentnet::network::behaviour::DecentNetworkBehaviour;
use decentnet::network::identity::Network;
use libp2p::Swarm;
use tungstenite::protocol::Message;

// type Tx = UnboundedSender<Message>;
// type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;

pub struct WebServer {
    host: String,
    port: u16,
    #[allow(dead_code)]
    network: Network,
}

impl WebServer {
    pub fn new(host: String, port: u16, network: Network) -> WebServer {
        WebServer {
            host,
            port,
            network,
        }
    }

    pub async fn start(&self) -> Box<(TcpStream, SocketAddr)> {
        let listener = TcpListener::bind(format!("{}:{}", self.host, self.port))
            .await
            .unwrap();
        Box::new(listener.accept().await.unwrap())
    }

    pub async fn handle_connection(
        &self,
        stream: &mut TcpStream,
        swarm: &mut Swarm<DecentNetworkBehaviour>,
    ) {
        let web_socket = tokio_tungstenite::accept_async(stream).await.unwrap();
        println!("WebSocket connection established");
        let (mut tx, mut rx) = unbounded::<Message>();
        let (mut outgoing, mut incoming) = web_socket.split();
        loop {
            tokio::select! {
                res = incoming.next() => {
                    if let Some(msg) = res {
                        if let Ok(msg) = msg {
                            tx.send(msg).await.unwrap();
                        } else {
                            println!("Error: {:?}", msg);
                        }
                    }
                },
                msg = rx.next() => {
                    if let Some(msg) = msg {
                        let result = serde_json::from_str::<serde_json::Value>(msg.to_text().unwrap());
                        if let Ok(value) = result {
                            if let serde_json::Value::Object(obj)  = value {
                                if let Some(cmd) = obj.get("command") {
                                    let mut argsd: Vec<Value> = vec![];
                                    if let Some(serde_json::Value::Array(args)) = obj.get("args") {
                                            for arg in args {
                                                    argsd.push(arg.clone());
                                            }
                                    }
                                    if let serde_json::Value::String(cmd) = cmd {
                                        self.handle_cmd(cmd, argsd, swarm, &mut outgoing).await;
                                    }
                                }
                            } else {
                                let a = serde_json::from_str::<serde_json::Value>(msg.to_text().unwrap()).unwrap();
                                println!("{:?}", a);
                            }
                        } else {
                            println!("{:?}", result);
                            return;
                        }
                    } else {
                        println!("WebSocket connection closed");
                    }
                },
            }
        }
    }

    pub async fn handle_cmd(
        &self,
        cmd: &str,
        args: Vec<Value>,
        swarm: &mut Swarm<DecentNetworkBehaviour>,
        outgoing: &mut SplitSink<WebSocketStream<&mut TcpStream>, Message>,
    ) {
        match cmd {
            "node_id" => {
                let nodes = swarm.local_peer_id().to_base58();
                let res = json!({
                    "command": "node_id",
                    "result": nodes,
                });
                let msg = serde_json::to_string(&res).unwrap();
                outgoing.send(Message::text(msg)).await.unwrap();
            }
            "list_nodes" => {
                let nodes = list_nodes(swarm);
                let res = json!({
                    "command": "list_nodes",
                    "result": nodes,
                });
                let msg = serde_json::to_string(&res).unwrap();
                outgoing.send(Message::text(msg)).await.unwrap();
            }
            "get_nodes_from_bootnodes" => {
                let _ = get_nodes_from_bootnodes(swarm);
                let res = json!({
                    "command": "get_nodes_from_bootnodes",
                    "result": "Gettings nodes form bootnode",
                });
                let msg = serde_json::to_string(&res).unwrap();
                outgoing.send(Message::text(msg)).await.unwrap();
            }
            "get_nodes" => {
                if let Some(serde_json::Value::String(addr)) = args.first() {
                    let _ = get_nodes(swarm, addr);
                    let res = json!({
                        "command": "get_nodes",
                        "result": "Gettings nodes from ID",
                    });
                    let msg = serde_json::to_string(&res).unwrap();
                    outgoing.send(Message::text(msg)).await.unwrap();
                } else {
                    let res = json!({
                        "command": "get_nodes",
                        "result": "Missing or Invalid args",
                    });
                    let msg = serde_json::to_string(&res).unwrap();
                    outgoing.send(Message::text(msg)).await.unwrap();
                }
            }
            "dial_addr" => {
                if let Some(serde_json::Value::String(addr)) = args.first() {
                    let _ = dial_addr(swarm, addr);
                    let res = json!({
                        "command": "dial_addr",
                        "result": "Dialing Address",
                    });
                    let msg = serde_json::to_string(&res).unwrap();
                    outgoing.send(Message::text(msg)).await.unwrap();
                } else {
                    let res = json!({
                        "command": "dial_addr",
                        "result": "Missing or Invalid args",
                    });
                    let msg = serde_json::to_string(&res).unwrap();
                    outgoing.send(Message::text(msg)).await.unwrap();
                }
            }
            "ping" => {
                if let Some(serde_json::Value::String(addr)) = args.first() {
                    let _ = ping(swarm.behaviour_mut(), addr);
                    let res = json!({
                        "command": "ping",
                        "result": "Pinging Id",
                    });
                    let msg = serde_json::to_string(&res).unwrap();
                    outgoing.send(Message::text(msg)).await.unwrap();
                } else {
                    let res = json!({
                        "command": "ping",
                        "result": "Missing or Invalid args",
                    });
                    let msg = serde_json::to_string(&res).unwrap();
                    outgoing.send(Message::text(msg)).await.unwrap();
                }
            }
            "external_addresses" => {
                let addr = external_addresses(swarm);
                let res = json!({
                    "command": "external_addresses",
                    "result": addr,
                });
                let msg = serde_json::to_string(&res).unwrap();
                outgoing.send(Message::text(msg)).await.unwrap();
            }
            "bootnodes" => {
                let addr = bootnodes(swarm);
                let res = json!({
                    "command": "bootnodes",
                    "result": addr,
                });
                let msg = serde_json::to_string(&res).unwrap();
                outgoing.send(Message::text(msg)).await.unwrap();
            }
            "listeners" => {
                let addr = listeners(swarm);
                let res = json!({
                    "command": "listeners",
                    "result": addr,
                });
                let msg = serde_json::to_string(&res).unwrap();
                outgoing.send(Message::text(msg)).await.unwrap();
            }
            _ => {
                let res = json!({
                    "command": cmd,
                    "result": "Unknown command",
                });
                let msg = serde_json::to_string(&res).unwrap();
                outgoing.send(Message::text(msg)).await.unwrap();
            }
        }
    }

    pub async fn _handle_connection_old(
        &self,
        stream: &mut TcpStream,
        swarm: &mut Swarm<DecentNetworkBehaviour>,
    ) {
        let mut buf = [0u8; 20480];
        stream.read_exact(&mut buf).await.unwrap();

        let get = b"GET / HTTP/1.1\r\n";
        let log = b"GET /log HTTP/1.1\r\n";

        let (status, contents) = if buf.starts_with(get) {
            let mut contents = fs::read_to_string("media/index.html").unwrap();

            let ids = [
                "{network_id}",
                "{listeners}",
                "{external_address}",
                "{known_peers}",
            ];
            let peers = {
                let st = self
                    .network
                    .get_known_nodes(swarm)
                    .iter_mut()
                    .map(|peer| format!("<li>{}:{:?}<li>", peer.0.to_base58(), peer.1))
                    .collect::<Vec<String>>();
                let mut peers: String = st
                    .iter()
                    .flat_map(|peer| {
                        let src = &peer[..];
                        src.chars()
                    })
                    .collect();
                peers.insert_str(0, "<ul>");
                peers.push_str("</ul>");
                peers
            };

            let external_address = {
                let res = swarm
                    .external_addresses()
                    .into_iter()
                    .map(|addr| format!("<li>{}<li>", addr.addr))
                    .collect::<Vec<String>>();

                let mut res = res
                    .iter()
                    .flat_map(|res| {
                        let src = &res[..];
                        src.chars()
                    })
                    .collect::<String>();
                res.insert_str(0, "<ul>");
                res.push_str("</ul>");
                res
            };

            let listeners = {
                let res = swarm
                    .listeners()
                    .into_iter()
                    .map(|addr| format!("<li>{}<li>", addr))
                    .collect::<Vec<String>>();

                let mut res = res
                    .iter()
                    .flat_map(|res| {
                        let src = &res[..];
                        src.chars()
                    })
                    .collect::<String>();
                res.insert_str(0, "<ul>");
                res.push_str("</ul>");
                res
            };
            let id_conts = [
                swarm.local_peer_id().to_base58(),
                listeners,
                external_address,
                peers,
            ];

            for (id, cont) in ids.iter().zip(id_conts.iter()) {
                let id = id.to_string();
                let cont = cont.to_string();
                contents = contents.replace(&id, &cont);
            }

            ("HTTP/1.1 200 OK\r\n", contents)
        } else if buf.starts_with(log) {
            let mut contents = fs::read_to_string("media/log.html").unwrap();
            let log_content = fs::read_to_string("data/decentnet.log").unwrap();
            let log_content = log_content.replace('\n', "<br>");
            contents = contents.replace("{log}", &log_content);
            ("HTTP/1.1 200 OK\r\n", contents)
        } else {
            let contents = fs::read_to_string("media/404.html").unwrap();
            ("HTTP/1.1 404 NOT FOUND\r\n", contents)
        };
        let response = format!(
            "{}Content-Length: {}\r\n\r\n{}",
            status,
            contents.len(),
            contents
        );
        stream.write_all(response.as_bytes()).await.unwrap();
    }
}
