use std::{collections::HashMap, time::Duration};

use bevy::prelude::*;
use bevy_realtime::{
    payload::PresenceConfig,
    presence::{
        AppExtend as _, PrescenceTrack, PresenceEvent, PresenceForwarder, PresencePayloadEvent,
        PresenceState,
    },
    BuildChannel, ChannelBuilder, RealtimeClient, RealtimeClientBuilder, RealtimePlugin,
};

#[allow(dead_code)]
#[derive(Event, Debug, Default)]
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

#[derive(Resource)]
pub struct TestTimer(pub Timer);

fn main() {
    let client = RealtimeClientBuilder::new(
        "http://127.0.0.1:54321",
        std::env::var("SUPABASE_LOCAL_ANON_KEY").unwrap(),
    )
    .connect()
    .to_sync();

    let mut app = App::new();

    app.add_plugins(DefaultPlugins)
        .add_plugins((RealtimePlugin { client },))
        .add_systems(Startup, (setup,))
        .add_systems(Update, (evr_presence).chain())
        .add_presence_event::<ExPresenceEvent, ChannelBuilder>();

    app.run()
}
fn setup(mut commands: Commands, mut client: ResMut<RealtimeClient>) {
    commands.spawn(Camera2dBundle::default());

    let mut channel = client.channel("test");

    channel.0.set_presence_config(PresenceConfig {
        key: Some("TestPresKey".into()),
    });

    let mut c = commands.spawn(channel);

    let mut payload = HashMap::new();

    payload.insert("Location".into(), "UK".into());

    c.insert(PrescenceTrack { payload });

    c.insert(PresenceForwarder::<ExPresenceEvent>::new(
        PresenceEvent::Join,
    ));

    c.insert(BuildChannel);

    commands.insert_resource(TestTimer(Timer::new(
        Duration::from_secs(1),
        TimerMode::Repeating,
    )));
}

fn evr_presence(mut evr: EventReader<ExPresenceEvent>) {
    for ev in evr.read() {
        println!("Presence got! {:?}", ev);
    }
}
