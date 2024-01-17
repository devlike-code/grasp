use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct ComponentCategory {
    pub name: String,
    pub components: Vec<String>,
    pub hidden: bool,
}
