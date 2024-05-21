use bevy::prelude::*;
use bevy_gotrue::{just_logged_in, AuthCreds, AuthPlugin, Client as AuthClient};
use bevy_http_client::HttpClientPlugin;
use bevy_realtime::{
    message::{
        payload::{PostgresChangesEvent, PostgresChangesPayload},
        postgres_change_filter::PostgresChangeFilter,
    },
    postgres_changes::bevy::{PostgresForwarder, PostgresPayloadEvent, PostresEventApp as _},
    BevyChannelBuilder, BuildChannel, Client as RealtimeClient, RealtimePlugin,
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
        .add_plugins((
            HttpClientPlugin,
            RealtimePlugin::new(
                "http://127.0.0.1:54321/realtime/v1".into(),
                std::env::var("SUPABASE_LOCAL_ANON_KEY").unwrap(),
            ),
            AuthPlugin {
                endpoint: "http://127.0.0.1:54321/auth/v1".into(),
            },
        ))
        .add_systems(Startup, (setup,))
        .add_systems(Update, (evr_postgres, signed_in.run_if(just_logged_in)))
        .add_postgres_event::<ExPostgresEvent, BevyChannelBuilder>();

    app.run()
}

fn setup(mut commands: Commands, client: Res<RealtimeClient>, auth: Res<AuthClient>) {
    commands.spawn(Camera2dBundle::default());

    auth.sign_in(
        &mut commands,
        AuthCreds {
            id: "test@example.com".into(),
            password: "password".into(),
        },
    );

    let mut channel = client.channel();

    channel.topic("test");

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

fn signed_in(client: Res<RealtimeClient>, auth: Res<AuthClient>) {
    client
        .set_access_token(auth.access_token.clone().unwrap())
        .unwrap();
}

fn evr_postgres(mut evr: EventReader<ExPostgresEvent>) {
    for ev in evr.read() {
        println!("Change got! {:?}", ev);
    }
}
