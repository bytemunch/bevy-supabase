use bevy::ecs::system::{Commands, Resource, SystemId};
use ehttp::Headers;

use crate::{AuthCreds, Builder};

#[derive(Resource)]
pub struct Client {
    pub sign_in: SystemId<AuthCreds>,
    pub endpoint: String,
    pub headers: Headers,
}

impl Client {
    pub fn builder(&self) -> Builder {
        Builder {
            url: self.endpoint.clone(),
            headers: self.headers.clone(),
        }
    }
    pub fn insert_header(&mut self, key: impl ToString, value: impl ToString) -> &mut Self {
        self.headers.insert(key, value);
        self
    }
    /// Returns a URL for provider oauth flow
    pub fn get_url_for_provider(&self, provider: &str) -> String {
        format!("{}/authorize?provider={}", self.endpoint, provider)
    }
    pub fn sign_in(&self, commands: &mut Commands, creds: AuthCreds) {
        commands.run_system_with_input(self.sign_in, creds)
    }
}
