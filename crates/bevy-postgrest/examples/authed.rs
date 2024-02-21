use std::time::Duration;

use bevy::prelude::*;
use bevy_gotrue::{AuthClient, AuthPlugin, AuthSession, EmailOrPhone};
use bevy_postgrest::{Client, PostgresRequest, PostgresResponse, PostgrestPlugin};

#[derive(Resource)]
pub struct TestTimer(pub Timer);

fn main() {
    let endpoint = "http://127.0.0.1:54321/rest/v1/".into();

    let mut app = App::new();

    app.add_plugins(DefaultPlugins)
        .add_plugins(PostgrestPlugin { endpoint })
        .add_plugins(AuthPlugin {
            endpoint: "http://172.18.0.4:9999".into(),
        })
        .add_systems(Startup, (setup,))
        .add_systems(Update, (send_every_second,));

    app.run()
}

fn setup(mut commands: Commands, mut auth: ResMut<AuthClient>) {
    commands.insert_resource(TestTimer(Timer::new(
        Duration::from_secs(1),
        TimerMode::Repeating,
    )));

    auth.sign_in(
        &mut commands,
        EmailOrPhone::Email("test@example.com".into()),
        "password".into(),
    )
    .unwrap();
}

fn send_every_second(
    mut timer: ResMut<TestTimer>,
    time: Res<Time>,
    client: Res<Client>,
    mut evw: EventWriter<PostgresRequest<PostgresResponse>>,
    auth: Option<Res<AuthSession>>,
) {
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }
    let mut req = client.client.from("todos").select("*");

    if let Some(auth) = auth {
        req = req.auth(auth.0.access_token.clone());
    }

    evw.send(PostgresRequest::<PostgresResponse>::new(req));
}
