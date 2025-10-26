use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Player;

#[derive(Serialize, Deserialize)]
pub struct Interactable {
    pub kind: InteractableType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InteractableType {
    Door,
    Lever,
    Terminal,
}
