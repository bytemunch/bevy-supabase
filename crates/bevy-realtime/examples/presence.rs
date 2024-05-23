use std::collections::HashMap;

use bevy::prelude::*;
use bevy_realtime::{
    channel::ChannelBuilder,
    message::payload::PresenceConfig,
    presence::{
        bevy::{AppExtend as _, PrescenceTrack, PresenceForwarder, PresencePayloadEvent},
        PresenceEvent, PresenceState,
    },
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

fn setup(world: &mut World) {
    world.spawn(Camera2dBundle::default());

    let callback = world.register_system(build_channel_callback);
    let client = world.resource::<Client>();
    client.channel(callback).unwrap();
}

fn build_channel_callback(mut channel_builder: In<ChannelBuilder>, mut commands: Commands) {
    channel_builder
        .topic("test")
        .set_presence_config(PresenceConfig {
            key: Some("TestPresKey".into()),
        });

    let mut channel = commands.spawn(BevyChannelBuilder(channel_builder.0));

    let mut payload = HashMap::new();

    payload.insert("Location".into(), "UK".into());

    channel.insert(PrescenceTrack { payload });

    channel.insert(PresenceForwarder::<ExPresenceEvent>::new(
        PresenceEvent::Join,
    ));

    channel.insert(BuildChannel);
}

fn evr_presence(mut evr: EventReader<ExPresenceEvent>) {
    for ev in evr.read() {
        println!("Presence got! {:?}", ev);
    }
}
