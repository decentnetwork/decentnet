use lazy_static::lazy_static;
use tokio::{fs, io::AsyncWriteExt};

use super::{
    constants::{ANNONYMOUS_USER, NETWORK_ID, STORAGE_FILE_PATH, USER_EXISTS},
    models::{Recipe, Recipes},
};

pub async fn create_default_recipes() {
    if !*USER_EXISTS | *ANNONYMOUS_USER {
        let string = serde_json::to_string(&*DEFAULT_RECIPES).unwrap();
        let _result = fs::create_dir_all(format!("data/{}/", *NETWORK_ID)).await;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(&*STORAGE_FILE_PATH.clone())
            .await
            .unwrap();
        file.write_all(string.as_bytes()).await.unwrap();
    }
}

lazy_static! {
    pub static ref DEFAULT_RECIPES: Recipes = vec![
        Recipe {
            id: 0,
            name: "Coffee".to_owned(),
            ingredients: "Coffee".to_owned(),
            instructions: "Make Coffee".to_owned(),
            public: true
        },
        Recipe {
            id: 1,
            name: "Tea".to_owned(),
            ingredients: "Tea, Water".to_owned(),
            instructions: "Boil Water, add tea".to_owned(),
            public: true
        },
        Recipe {
            id: 2,
            name: "Carrot Cake".to_owned(),
            ingredients: "Carrot, Cake".to_owned(),
            instructions: "Make Carrot Cake".to_owned(),
            public: true
        },
        Recipe {
            id: 3,
            name: "Biryani".to_owned(),
            ingredients: "Rice, Water, Masala".to_owned(),
            instructions: "Cook Biryani".to_owned(),
            public: true
        },
    ];
}
