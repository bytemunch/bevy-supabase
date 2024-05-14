use std::{collections::HashMap, time::Duration};

use bevy::prelude::*;
use bevy_realtime::{
    broadcast::{AppExtend as _, BroadcastForwarder, BroadcastPayloadEvent},
    internal::message::payload::{BroadcastConfig, BroadcastPayload},
    BevyChannelBuilder, BuildChannel, Channel, Client, RealtimePlugin,
};
use serde_json::Value;

#[derive(Event, Debug, Default)]
pub struct ExBroadcastEvent {
    payload: HashMap<String, Value>,
}

impl BroadcastPayloadEvent for ExBroadcastEvent {
    fn new(payload: HashMap<String, Value>) -> Self {
        Self { payload }
    }
}

#[derive(Resource)]
pub struct TestTimer(pub Timer);

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins)
        .add_plugins((RealtimePlugin::new(
            "http://127.0.0.1:54321/realtime/v1".into(),
            std::env::var("SUPABASE_LOCAL_ANON_KEY").unwrap(),
        ),))
        .add_systems(Startup, (setup,))
        .add_systems(Update, (send_every_second, evr_broadcast).chain())
        .add_broadcast_event::<ExBroadcastEvent, BevyChannelBuilder>();

    app.run()
}

fn setup(mut commands: Commands, client: Res<Client>) {
    commands.spawn(Camera2dBundle::default());

    let mut channel = client.channel("test".into());

    channel.set_broadcast_config(BroadcastConfig {
        broadcast_self: true,
        ack: false,
    });

    let mut c = commands.spawn(BevyChannelBuilder(channel));

    c.insert(BroadcastForwarder::<ExBroadcastEvent>::new("test".into()));

    c.insert(BuildChannel);

    commands.insert_resource(TestTimer(Timer::new(
        Duration::from_secs(1),
        TimerMode::Repeating,
    )));
}

fn evr_broadcast(mut evr: EventReader<ExBroadcastEvent>) {
    for ev in evr.read() {
        println!("Broadcast got! {:?}", ev.payload);
    }
}

fn send_every_second(q_channel: Query<&Channel>, mut timer: ResMut<TestTimer>, time: Res<Time>) {
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }
    let mut payload = HashMap::new();
    payload.insert("bevy?".into(), "bavy.".into());
    for c in q_channel.iter() {
        c.broadcast(BroadcastPayload {
            event: "test".into(),
            payload: payload.clone(),
            ..Default::default()
        })
        .unwrap();
    }
}
