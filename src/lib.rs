// Glue plugin for gotrue, auth and postgrest

pub mod prelude;

use bevy::prelude::*;

use bevy_gotrue::{AuthPlugin, Session};
use bevy_postgrest::PostgrestPlugin;
use bevy_realtime::RealtimePlugin;

pub use bevy_gotrue::{is_logged_in, just_logged_in, Client as AuthClient};
pub use bevy_postgrest::Client as PostgrestClient;
pub use bevy_realtime::Client as RealtimeClient;

#[derive(Resource)]
pub struct SupabaseClient {
    pub apikey: String,
    pub endpoint: String,
}

#[derive(Default)]
pub struct SupabasePlugin {
    pub endpoint: String,
    pub apikey: String,
    pub auth_endpoint: Option<String>,
    pub postgrest_endpoint: Option<String>,
    pub realtime_endpoint: Option<String>,
}

impl Plugin for SupabasePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            //AuthPlugin::new(
            //    self.auth_endpoint
            //        .clone()
            //        .unwrap_or(format!("{}/auth/v1", self.endpoint)),
            //),
            PostgrestPlugin::new(
                self.postgrest_endpoint
                    .clone()
                    .unwrap_or(format!("{}/rest/v1", self.endpoint)),
            ),
            RealtimePlugin::new(
                self.realtime_endpoint
                    .clone()
                    .unwrap_or(format!("{}/realtime/v1", self.endpoint)),
                self.apikey.clone(),
            ),
        ))
        .insert_resource(SupabaseClient {
            apikey: self.apikey.clone(),
            endpoint: self.endpoint.clone(),
        })
        .add_systems(Startup, (setup_apikey,))
        .add_systems(
            Update,
            (
                (update_realtime_access_token, update_postgrest_access_token)
                    .run_if(just_logged_in),
            ),
        );
    }
}

fn setup_apikey(
    supabase_client: Res<SupabaseClient>,
    mut db_client: ResMut<PostgrestClient>,
    //mut auth_client: ResMut<AuthClient>,
) {
    // Add apikey headers to all internal plugins
    // Realtime is initialized with an api key so not needed here
    let apikey = supabase_client.apikey.clone();
    db_client.insert_header("apikey", apikey.clone());
    //auth_client.insert_header("apikey", apikey.clone());
}

fn update_realtime_access_token(client: Res<RealtimeClient>, auth: Res<Session>) {
    client
        .0
        .set_access_token(auth.access_token.clone())
        .unwrap();
}

fn update_postgrest_access_token(mut client: ResMut<PostgrestClient>, auth: Res<Session>) {
    client.insert_header("Authorization", auth.access_token.clone());
}
