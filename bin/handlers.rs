use std::collections::HashSet;

use super::{
    constants::{STORAGE_FILE_PATH, TOPIC},
    models::{ListBroadcast, ListMode, ListRequest, ListResponse, Recipe, Recipes, Result},
};
use decentnet::network::behaviour::DecentNetworkBehaviour;
use libp2p::{PeerId, Swarm};
use log::{error, info};
use tokio::{
    fs,
    sync::mpsc::{self, UnboundedSender},
};

//TODO? : Move below fn to core.
pub fn handle_list_nodes(swarm: &mut Swarm<DecentNetworkBehaviour>, show_addresses: bool) {
    info!("Discovered Nodes:");
    DecentNetworkBehaviour::list_nodes(swarm)
        .into_iter()
        .for_each(|node| {
            let mut res = format!("Network Id : {:?}", node.network_id);
            if show_addresses {
                res.push_str(&format!(", Address : {:?}", node.multiaddr));
            }
            info!("{}", res);
        });
}

pub async fn handle_list_recipes(cmd: &str, swarm: &mut Swarm<DecentNetworkBehaviour>) {
    let rest = cmd.strip_prefix("ls r ");
    match rest {
        Some("all") => {
            let req = ListRequest {
                mode: ListMode::All,
            };
            let json = serde_json::to_string(&req).expect("can jsonify request");
            swarm
                .behaviour_mut()
                .floodsub
                .publish(TOPIC.clone(), json.as_bytes());
        }
        Some(recipes_network_id) => {
            let req = ListRequest {
                mode: ListMode::One(recipes_network_id.to_owned()),
            };
            let json = serde_json::to_string(&req).expect("can jsonify request");
            swarm
                .behaviour_mut()
                .floodsub
                .publish(TOPIC.clone(), json.as_bytes());
        }
        None => {
            match read_local_recipes().await {
                Ok(v) => {
                    info!("Local Recipes ({})", v.len());
                    v.iter().for_each(|r| info!("{}", r));
                }
                Err(e) => error!("error fetching local recipes: {}", e),
            };
        }
    };
}

pub async fn handle_create_recipe(cmd: &str) {
    let cmd_pos = cmd.find(" r").unwrap();
    let rest = cmd.split_at(cmd_pos + 3).1;
    info!("{}", rest);
    let elements: Vec<&str> = rest.split('|').collect();
    if elements.len() < 3 {
        info!("too few arguments - Format: name|ingredients|instructions");
    } else {
        let name = elements.get(0).expect("name is there");
        let ingredients = elements.get(1).expect("ingredients is there");
        let instructions = elements.get(2).expect("instructions is there");
        if let Err(e) = create_new_recipe(name, ingredients, instructions).await {
            error!("error creating recipe: {}", e);
        };
    }
}

pub async fn handle_delete_recipe(cmd: &str) {
    let cmd_pos = cmd.find(" r").unwrap();
    let rest = cmd.split_at(cmd_pos + 3).1;
    let id = rest.parse::<usize>().expect("can parse id");
    if let Err(e) = delete_recipe(id).await {
        error!("error deleting recipe: {}", e);
    };
}

pub async fn handle_publish_recipe(cmd: &str) {
    let cmd_pos = cmd.find(" r").unwrap();
    let rest = cmd.split_at(cmd_pos + 3).1;
    match rest.trim().parse::<usize>() {
        Ok(id) => {
            if let Err(e) = publish_recipe(id).await {
                info!("error publishing recipe with id {}, {}", id, e)
            } else {
                info!("Published Recipe with id: {}", id);
            }
        }
        Err(e) => error!("invalid id: {}, {}", rest.trim(), e),
    };
}

pub async fn handle_broadcast_recipe(
    swarm: &mut Swarm<DecentNetworkBehaviour>,
    sender: UnboundedSender<ListBroadcast>,
) {
    info!("Discovered Nodes:");
    let mut unique_nodes = HashSet::<&PeerId>::new();
    let nodes = swarm.behaviour().mdns.as_ref().unwrap().discovered_nodes();
    for node in nodes {
        unique_nodes.insert(node);
    }
    unique_nodes.iter().for_each(|p| info!("{}", p));

    for node in unique_nodes {
        broadcast_recipe(node.to_base58(), sender.clone())
            .await
            .expect("can send broadcast");
    }
}

async fn create_new_recipe(name: &str, ingredients: &str, instructions: &str) -> Result<()> {
    let mut local_recipes = read_local_recipes().await?;
    let new_id = match local_recipes.iter().max_by_key(|r| r.id) {
        Some(v) => v.id + 1,
        None => 0,
    };
    local_recipes.push(Recipe {
        id: new_id,
        name: name.to_owned(),
        ingredients: ingredients.to_owned(),
        instructions: instructions.to_owned(),
        public: false,
    });
    write_local_recipes(&local_recipes).await?;

    info!("Created recipe:");
    info!("Id: {}", new_id);
    info!("Name: {}", name);
    info!("Ingredients: {}", ingredients);
    info!("Instructions:: {}", instructions);

    Ok(())
}

async fn delete_recipe(id: usize) -> Result<()> {
    let mut local_recipes = read_local_recipes().await?;
    local_recipes.retain(|r| r.id != id);
    write_local_recipes(&local_recipes).await?;
    info!("Deleting recipe:");
    info!("Id: {}", id);

    Ok(())
}

async fn publish_recipe(id: usize) -> Result<()> {
    let mut local_recipes = read_local_recipes().await?;
    local_recipes
        .iter_mut()
        .filter(|r| r.id == id)
        .for_each(|r| r.public = true);
    write_local_recipes(&local_recipes).await?;
    Ok(())
}

async fn broadcast_recipe(receiver: String, sender: UnboundedSender<ListBroadcast>) -> Result<()> {
    info!("Broadcasting recipe to {}", receiver);
    let local_recipes = read_local_recipes().await?;
    let local_recipes = local_recipes.into_iter().filter(|r| r.public).collect();
    let resp = ListBroadcast {
        receiver,
        data: local_recipes,
    };
    sender.send(resp).expect("can send broadcast");
    Ok(())
}

pub async fn read_local_recipes() -> Result<Recipes> {
    let path = STORAGE_FILE_PATH.to_string();
    // info!("Reading local recipes from {:?}", path);
    let content = fs::read(path).await?;
    let result = serde_json::from_slice(&content)?;
    Ok(result)
}

pub async fn write_local_recipes(recipes: &Recipes) -> Result<()> {
    let json = serde_json::to_string(&recipes)?;
    fs::write(STORAGE_FILE_PATH.to_string(), &json).await?;
    Ok(())
}

pub fn respond_with_public_recipes(sender: mpsc::UnboundedSender<ListResponse>, receiver: String) {
    tokio::spawn(async move {
        match read_local_recipes().await {
            Ok(recipes) => {
                let resp = ListResponse {
                    mode: ListMode::All,
                    receiver,
                    data: recipes.into_iter().filter(|r| r.public).collect(),
                };
                if let Err(e) = sender.send(resp) {
                    error!("error sending response via channel, {}", e);
                }
            }
            Err(e) => error!("error fetching local recipes to answer ALL request, {}", e),
        }
    });
}
