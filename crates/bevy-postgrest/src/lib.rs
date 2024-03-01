mod builder;
mod client;
pub mod filter;

use bevy::prelude::*;
pub use client::Client;

pub struct PostgrestPlugin {
    pub endpoint: String,
}

impl PostgrestPlugin {
    pub fn new(endpoint: String) -> Self {
        Self { endpoint }
    }
}

impl Plugin for PostgrestPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Client::new(self.endpoint.clone()));
    }
}
