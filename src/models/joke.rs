#[derive(serde::Deserialize)]
pub struct Joke {
    pub joke: Option<String>,
    pub setup: Option<String>,
    pub delivery: Option<String>,
}
