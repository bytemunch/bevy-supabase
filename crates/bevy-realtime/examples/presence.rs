use std::{collections::HashMap, time::Duration};

use bevy::{ecs::system::SystemId, prelude::*, time::common_conditions::on_timer};
use bevy_realtime::{
    channel::ChannelBuilder,
    message::payload::PresenceConfig,
    presence::{
        bevy::{AppExtend as _, PrescenceTrack, PresenceForwarder, PresencePayloadEvent},
        PresenceEvent, PresenceState,
    },
    BevyChannelBuilder, BuildChannel, Channel, Client, RealtimePlugin,
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
        .add_systems(
            Update,
            (
                evr_presence,
                test_get_presence_state.run_if(on_timer(Duration::from_secs(1))),
            ),
        )
        .add_presence_event::<ExPresenceEvent, BevyChannelBuilder>();

    app.run()
}

fn setup(world: &mut World) {
    world.spawn(Camera2dBundle::default());

    let callback = world.register_system(build_channel_callback);
    let client = world.resource::<Client>();
    client.channel(callback).unwrap();

    let test_callback = world.register_system(get_presence_state);
    world.insert_resource(TestCallback(test_callback));
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

#[derive(Resource, Deref)]
struct TestCallback(pub SystemId<PresenceState>);

fn test_get_presence_state(channel: Query<&Channel>, callback: Res<TestCallback>) {
    for c in channel.iter() {
        c.presence_state(**callback).unwrap();
    }
}

fn get_presence_state(state: In<PresenceState>) {
    println!("State got! {:?}", *state);
}

fn evr_presence(mut evr: EventReader<ExPresenceEvent>) {
    for ev in evr.read() {
        println!("Presence got! {:?}", ev);
    }
}
