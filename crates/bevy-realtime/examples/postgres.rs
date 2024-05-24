use bevy::prelude::*;
use bevy_realtime::{
    channel::ChannelBuilder,
    message::{
        payload::{PostgresChangesEvent, PostgresChangesPayload},
        postgres_change_filter::PostgresChangeFilter,
    },
    postgres_changes::bevy::{PostgresForwarder, PostgresPayloadEvent, PostresEventApp as _},
    BevyChannelBuilder, BuildChannel, Client, RealtimePlugin,
};

#[allow(dead_code)]
#[derive(Event, Debug, Clone)]
pub struct ExPostgresEvent {
    payload: PostgresChangesPayload,
}

impl PostgresPayloadEvent for ExPostgresEvent {
    fn new(payload: PostgresChangesPayload) -> Self {
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
        .add_systems(Update, (evr_postgres).chain())
        .add_postgres_event::<ExPostgresEvent, BevyChannelBuilder>();

    app.run()
}

fn setup(world: &mut World) {
    world.spawn(Camera2dBundle::default());

    let callback = world.register_system(build_channel_callback);
    let client = world.resource::<Client>();
    client.channel(callback).unwrap();
}

fn build_channel_callback(mut channel_builder: In<ChannelBuilder>, mut commands: Commands) {
    channel_builder.topic("test");

    let mut channel = commands.spawn(BevyChannelBuilder(channel_builder.0));

    channel.insert(PostgresForwarder::<ExPostgresEvent>::new(
        PostgresChangesEvent::All,
        PostgresChangeFilter {
            schema: "public".into(),
            table: Some("todos".into()),
            filter: None,
        },
    ));

    channel.insert(BuildChannel);
}

fn evr_postgres(mut evr: EventReader<ExPostgresEvent>) {
    for ev in evr.read() {
        println!("Change got! {:?}", ev);
    }
}
