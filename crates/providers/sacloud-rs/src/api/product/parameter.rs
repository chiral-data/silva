use serde::Serialize;

#[derive(Serialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct Params {
    from: usize,
    count: usize,
}

impl Params {
    pub fn from(mut self, from: usize) -> Self {
        self.from = from; 
        self
    }

    pub fn count(mut self, count: usize) -> Self {
        self.count = count;
        self
    }
}

impl From<Params> for Vec<(&str, String)> {
    fn from(value: Params) -> Self {
        vec![
            ("From", value.from.to_string()),
            ("Count", value.count.to_string())
        ]
    }
} 
