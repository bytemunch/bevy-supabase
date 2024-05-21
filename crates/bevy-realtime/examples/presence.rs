use std::collections::HashMap;

use bevy::prelude::*;
use bevy_realtime::{
    message::payload::PresenceConfig,
    presence::bevy::{AppExtend as _, PrescenceTrack, PresenceForwarder, PresencePayloadEvent},
    presence::{PresenceEvent, PresenceState},
    BevyChannelBuilder, BuildChannel, Client, RealtimePlugin,
};

#[allow(dead_code)]
#[derive(Event, Debug, Default, Clone)]
pub struct ExPresenceEvent {
    key: String,
    new_state: PresenceState,
    old_state: PresenceState,
}

impl PresencePayloadEvent for ExPresenceEvent {
    fn new(key: String, old_state: PresenceState, new_state: PresenceState) -> Self {
        Self {
            key,
            new_state,
            old_state,
        }
    }
}

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins)
        .add_plugins((RealtimePlugin::new(
            "http://127.0.0.1:54321/realtime/v1".into(),
            std::env::var("SUPABASE_LOCAL_ANON_KEY").unwrap(),
        ),))
        .add_systems(Startup, (setup,))
        .add_systems(Update, (evr_presence).chain())
        .add_presence_event::<ExPresenceEvent, BevyChannelBuilder>();

    app.run()
}
fn setup(mut commands: Commands, client: Res<Client>) {
    commands.spawn(Camera2dBundle::default());

    let mut channel = client.channel();

    channel.topic("test").set_presence_config(PresenceConfig {
        key: Some("TestPresKey".into()),
    });

    let mut c = commands.spawn(BevyChannelBuilder(channel));

    let mut payload = HashMap::new();

    payload.insert("Location".into(), "UK".into());

    c.insert(PrescenceTrack { payload });

    c.insert(PresenceForwarder::<ExPresenceEvent>::new(
        PresenceEvent::Join,
    ));

    c.insert(BuildChannel);
}

fn evr_presence(mut evr: EventReader<ExPresenceEvent>) {
    for ev in evr.read() {
        println!("Presence got! {:?}", ev);
    }
}
