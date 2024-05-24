use std::{collections::HashMap, time::Duration};

use bevy::{ecs::system::SystemId, prelude::*, time::common_conditions::on_timer};
use bevy_realtime::{
    broadcast::bevy::{BroadcastEventApp, BroadcastForwarder, BroadcastPayloadEvent},
    channel::{ChannelBuilder, ChannelState},
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

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins)
        .add_plugins((RealtimePlugin::new(
            "http://127.0.0.1:54321/realtime/v1".into(),
            std::env::var("SUPABASE_LOCAL_ANON_KEY").unwrap(),
        ),))
        .add_systems(Startup, (setup,))
        .add_systems(
            Update,
            ((
                (send_every_second, test_get_channel_state)
                    .run_if(on_timer(Duration::from_secs(1))),
                evr_broadcast,
            )
                .chain()
                .run_if(client_ready),),
        )
        .add_broadcast_event::<ExBroadcastEvent, BevyChannelBuilder>();

    app.run()
}

fn setup(world: &mut World) {
    println!("setup s1 ");

    world.spawn(Camera2dBundle::default());

    let callback = world.register_system(build_channel_callback);
    let client = world.resource::<Client>();

    client.channel(callback).unwrap();

    let test_callback = world.register_system(get_channel_state);
    world.insert_resource(TestCallback(test_callback));

    println!("setup s1 finished");
}

fn build_channel_callback(mut channel_builder: In<ChannelBuilder>, mut commands: Commands) {
    println!("channel setup s2 ");
    channel_builder
        .topic("test")
        .set_broadcast_config(BroadcastConfig {
            broadcast_self: true,
            ack: false,
        });

    let mut c = commands.spawn(BevyChannelBuilder(channel_builder.0));

    c.insert(BroadcastForwarder::<ExBroadcastEvent>::new("test".into()));

    c.insert(BuildChannel);
    println!("channel setup s2 finished");
}

fn evr_broadcast(mut evr: EventReader<ExBroadcastEvent>) {
    for ev in evr.read() {
        println!("Broadcast got! {:?}", ev.payload);
    }
}

#[derive(Resource, Deref)]
struct TestCallback(pub SystemId<ChannelState>);

fn test_get_channel_state(channel: Query<&Channel>, callback: Res<TestCallback>) {
    println!("Get state...");
    for c in channel.iter() {
        c.channel_state(**callback).unwrap();
    }
}

fn get_channel_state(state: In<ChannelState>) {
    println!("State got! {:?}", *state);
}

fn send_every_second(q_channel: Query<&Channel>) {
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
