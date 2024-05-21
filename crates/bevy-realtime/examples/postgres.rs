use bevy::prelude::*;
use bevy_realtime::{
    message::{
        payload::{PostgresChangesEvent, PostgresChangesPayload},
        postgres_change_filter::PostgresChangeFilter,
    },
    postgres_changes::bevy::{AppExtend as _, PostgresForwarder, PostgresPayloadEvent},
    BevyChannelBuilder, BuildChannel, Client, RealtimePlugin,
};

#[allow(dead_code)]
#[derive(Event, Debug)]
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
fn setup(mut commands: Commands, client: Res<Client>) {
    commands.spawn(Camera2dBundle::default());

    let channel = client.channel("test".into());

    let mut c = commands.spawn(BevyChannelBuilder(channel));

    c.insert(PostgresForwarder::<ExPostgresEvent>::new(
        PostgresChangesEvent::All,
        PostgresChangeFilter {
            schema: "public".into(),
            table: Some("todos".into()),
            filter: None,
        },
    ));

    c.insert(BuildChannel);
}

fn evr_postgres(mut evr: EventReader<ExPostgresEvent>) {
    for ev in evr.read() {
        println!("Change got! {:?}", ev);
    }
}
