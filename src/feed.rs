use std::collections::hashmap::HashMap;

pub struct Element {
    pub ty: String,
    pub fields: HashMap<String, ElementValue>,
}

impl Element {
    pub fn new(ty: String) -> Element {
        Element { ty: ty, fields: HashMap::new() }
    }
}

pub enum ElementValue {
    Str(String), Elem(Element)
}