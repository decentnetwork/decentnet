pub mod api;
pub mod network;

pub use api::{build_development_swarm, development_net_conf};

#[cfg(test)]
mod tests {
    use super::api::{build_development_swarm, development_net_conf};
    use libp2p::futures::StreamExt;
    use tokio::select;

    use crate::network::{behaviour::DecentNetworkBehaviour, identity::Network};

    #[tokio::test]
    async fn test_swarm() {
        let client_config1 = development_net_conf(26117, false);
        let client_config2 = development_net_conf(26118, true);

        let mut swarm1 = build_development_swarm(&client_config1).await;
        let mut swarm2 = build_development_swarm(&client_config2).await;

        let result1 = Network::start_listening(&mut swarm1, &client_config1);
        let result2 = Network::start_listening(&mut swarm2, &client_config2);
        if result1.0.is_err() || result2.0.is_err() {
            if let Err(e) = result1.0 {
                println!("Swarm 1 Error : {:?}", e);
            }
            if let Err(e) = result2.0 {
                println!("Swarm 2 Error : {:?}", e);
            }
        }

        loop {
            select! {
                event1 = swarm1.select_next_some() => {
                    match event1 {
                        event =>  DecentNetworkBehaviour::handle_swarm_event(&mut swarm1, event, &client_config1)
                    }
                },
                event2 = swarm2.select_next_some() => {
                    match event2 {
                        event =>  DecentNetworkBehaviour::handle_swarm_event(&mut swarm2, event, &client_config2)
                    }
                },
            };
        }
    }
}
