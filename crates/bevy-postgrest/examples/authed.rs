use std::time::Duration;

use bevy::prelude::*;
use bevy_gotrue::{AuthClient, AuthCreds, AuthPlugin, Session};
use bevy_http_client::HttpClientPlugin;
use bevy_postgrest::{api::Postgrest, AppExtend, PostgresRequest, PostgrestPlugin, ResponseEvent};

#[derive(Resource)]
pub struct TestTimer(pub Timer);

#[derive(Event, Debug)]
pub struct MyPostgresResponse {
    res: String,
}

impl ResponseEvent for MyPostgresResponse {
    fn new(res: String) -> Self {
        Self { res }
    }

    fn get_res(&self) -> String {
        self.res.clone()
    }
}

pub type MyPostgresRequest = PostgresRequest<MyPostgresResponse>;

fn main() {
    let endpoint = "http://127.0.0.1:54321/rest/v1/".into();

    let mut app = App::new();

    app.add_plugins(DefaultPlugins)
        .add_plugins(HttpClientPlugin)
        .add_plugins(PostgrestPlugin { endpoint })
        .add_plugins(AuthPlugin {
            endpoint: "http://127.0.0.1:54321/auth/v1".into(),
        })
        .add_systems(Startup, (setup,))
        .add_systems(
            Update,
            (
                send_every_second,
                read_postgres_responses::<MyPostgresResponse>,
            ),
        )
        .add_postgrest_event::<MyPostgresResponse>();

    app.run()
}

fn setup(mut commands: Commands, auth: Res<AuthClient>) {
    commands.insert_resource(TestTimer(Timer::new(
        Duration::from_secs(1),
        TimerMode::Repeating,
    )));

    auth.sign_in(
        &mut commands,
        AuthCreds {
            id: "test@example.com".into(),
            password: "password".into(),
        },
    );
}

fn send_every_second(
    mut timer: ResMut<TestTimer>,
    time: Res<Time>,
    client: Res<Postgrest>,
    mut evw: EventWriter<MyPostgresRequest>,
    auth: Option<Res<Session>>,
) {
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }
    let mut req = client.from("todos").select("*");

    if let Some(auth) = auth {
        req = req.auth(auth.access_token.clone());
    }

    evw.send(MyPostgresRequest::new(req));
}

fn read_postgres_responses<T: ResponseEvent + Event>(mut evr: EventReader<T>) {
    for ev in evr.read() {
        println!("{:?}", ev.get_res());
    }
}
