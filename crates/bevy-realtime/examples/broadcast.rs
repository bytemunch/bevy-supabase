use std::{collections::HashMap, time::Duration};

use bevy::prelude::*;
use bevy_realtime::{
    broadcast::bevy::{BroadcastEventApp, BroadcastForwarder, BroadcastPayloadEvent},
    client_ready,
    message::payload::{BroadcastConfig, BroadcastPayload},
    BevyChannelBuilder, BuildChannel, Channel, Client, RealtimePlugin,
};
use serde_json::Value;

#[derive(Event, Debug, Default, Clone)]
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
        .add_systems(Startup, (bevy_setup,))
        .add_systems(
            Update,
            (
                (tick_timer, say_cheese),
                (send_every_second, evr_broadcast, setup_channels)
                    .chain()
                    .run_if(client_ready),
            ),
        )
        .add_broadcast_event::<ExBroadcastEvent, BevyChannelBuilder>();

    app.run()
}

fn setup_channels(mut commands: Commands, client: Res<Client>, mut has_run: Local<bool>) {
    if *has_run {
        return;
    }

    println!("running channel setup");

    *has_run = true;

    let mut channel = client.channel("test".into());

    channel.set_broadcast_config(BroadcastConfig {
        broadcast_self: true,
        ack: false,
    });

    let mut c = commands.spawn(BevyChannelBuilder(channel));

    c.insert(BroadcastForwarder::<ExBroadcastEvent>::new("test".into()));

    c.insert(BuildChannel);

    println!("setup finished");
}

fn bevy_setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
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

fn tick_timer(mut timer: ResMut<TestTimer>, time: Res<Time>) {
    timer.0.tick(time.delta());
}

fn say_cheese(timer: Res<TestTimer>, mut count: Local<usize>) {
    if !timer.0.just_finished() {
        return;
    }
    info!("che{}ese", "e".repeat(*count));
    *count += 1;
}

fn send_every_second(q_channel: Query<&Channel>, timer: Res<TestTimer>) {
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
