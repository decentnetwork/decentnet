use serde::{Deserialize, Serialize};
use std::fmt;

pub type Recipes = Vec<Recipe>;
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;

#[derive(Debug, Deserialize, Serialize)]
pub struct Recipe {
    pub id: usize,
    pub name: String,
    pub ingredients: String,
    pub instructions: String,
    pub public: bool,
}

impl fmt::Display for Recipe {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Recipe(Id: {}, Name: {}, Ingredients: {}, Instructions: {})",
            self.id, self.name, self.ingredients, self.instructions
        )
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum ListMode {
    All,
    One(String),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ListRequest {
    pub mode: ListMode,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ListResponse {
    pub mode: ListMode,
    pub data: Recipes,
    pub receiver: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ListBroadcast {
    pub data: Recipes,
    pub receiver: String,
}

pub enum EventType {
    Broadcast(ListBroadcast),
    Response(ListResponse),
    Input(String),
}
