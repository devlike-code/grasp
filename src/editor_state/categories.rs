#[derive(Default, Debug, Clone)]
pub struct ComponentEntry {
    pub name: String,
    pub display: String,
    pub hidden: bool,
}

#[derive(Default, Debug, Clone)]
pub struct ComponentCategory {
    pub name: String,
    pub components: Vec<ComponentEntry>,
    pub hidden: bool,
}