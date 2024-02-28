pub mod api;
pub mod builder;
pub mod filter;

use api::Postgrest;
use bevy::prelude::*;

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
        app.insert_resource(Postgrest::new(self.endpoint.clone()));
    }
}
