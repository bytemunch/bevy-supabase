use bevy::{
    app::{App, Plugin},
    ecs::system::Resource,
};
pub use realtime_rs::{message::*, realtime_channel::*, realtime_client::*, realtime_presence::*};

#[derive(Resource)]
pub struct RealtimeClientManagerResource(pub ClientManagerSync);

#[derive(Default)]
pub struct RealtimePlugin {
    pub client: Option<ClientManagerSync>,
    pub endpoint: Option<String>,
    pub anon_key: Option<String>,
}

impl Plugin for RealtimePlugin {
    fn build(&self, app: &mut App) {
        match self.client {
            Some(ref client) => {
                app.insert_resource(RealtimeClientManagerResource(client.clone()));
            }
            None => {
                app.insert_resource(RealtimeClientManagerResource(
                    RealtimeClientBuilder::new(
                        self.endpoint.clone().unwrap(),
                        self.anon_key.clone().unwrap(),
                    )
                    .connect()
                    .to_sync(),
                ));
            }
        }
        println!("We plugged in");
    }
}
