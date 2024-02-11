use std::{collections::HashMap, time::Duration};

use bevy::prelude::*;
use bevy_supabase_realtime::{
    payload::{BroadcastConfig, BroadcastPayload},
    ChannelManagerSync, RealtimeClientManagerResource, RealtimePlugin,
};
use serde_json::Value;
use tokio::sync::mpsc::{self, Receiver};

#[derive(Event, Debug)]
pub struct BroadcastEvent {
    event: String,
    payload: HashMap<String, Value>,
}

#[derive(Resource)]
pub struct BroadcastRxs(pub Vec<Receiver<BroadcastEvent>>);

#[derive(Resource)]
pub struct TestChannel(pub ChannelManagerSync);

#[derive(Resource)]
pub struct TestTimer(pub Timer);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((RealtimePlugin {
            endpoint: Some("http://127.0.0.1:54321".into()),
            anon_key: Some(std::env::var("SUPABASE_LOCAL_ANON_KEY").unwrap()),
            ..Default::default()
        },))
        .add_systems(Startup, (setup,))
        .add_systems(Update, (poll_broadcast_rxs, evr_broadcast).chain())
        .add_systems(Update, send_every_second)
        .add_event::<BroadcastEvent>()
        .run()
}

fn setup(mut commands: Commands, client: ResMut<RealtimeClientManagerResource>) {
    commands.spawn(Camera2dBundle::default());

    let (tx, rx) = mpsc::channel(255);

    let c = client
        .0
        .channel("test")
        .broadcast(BroadcastConfig {
            broadcast_self: true,
            ack: false,
        })
        .on_broadcast("test", move |payload| {
            tx.try_send(BroadcastEvent {
                payload: payload.clone(),
                event: "test".into(),
            })
            .unwrap();
        })
        .build_sync(&client.0)
        .unwrap();

    c.subscribe_blocking().unwrap();

    let mut rxs = Vec::new();

    rxs.push(rx);

    commands.insert_resource(BroadcastRxs(rxs));
    commands.insert_resource(TestChannel(c));
    commands.insert_resource(TestTimer(Timer::new(
        Duration::from_secs(1),
        TimerMode::Repeating,
    )));
}

fn poll_broadcast_rxs(mut rxs: ResMut<BroadcastRxs>, mut evw: EventWriter<BroadcastEvent>) {
    for rx in rxs.0.iter_mut() {
        match rx.try_recv() {
            Ok(ev) => evw.send(ev),
            Err(ref e) if *e == tokio::sync::mpsc::error::TryRecvError::Empty => continue,
            Err(e) => println!("Channel recv error: {:?}", e),
        }
    }
}

fn evr_broadcast(mut evr: EventReader<BroadcastEvent>) {
    for ev in evr.read() {
        println!("Broadcast got! {:?}", ev);
    }
}

fn send_every_second(channel: Res<TestChannel>, mut timer: ResMut<TestTimer>, time: Res<Time>) {
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }
    let mut payload = HashMap::new();
    payload.insert("bevy?".into(), "bavy.".into());
    channel.0.broadcast(BroadcastPayload {
        event: "test".into(),
        payload,
        ..Default::default()
    });
}
