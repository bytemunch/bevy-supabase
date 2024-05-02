use std::time::Duration;

use bevy::{prelude::*, time::common_conditions::on_timer};
use bevy_gotrue::{is_logged_in, AuthCreds, AuthPlugin, Client as AuthClient};
use bevy_http_client::{
    prelude::{HttpTypedRequestTrait, TypedRequest, TypedResponse, TypedResponseError},
    HttpClient, HttpClientPlugin,
};
use bevy_postgrest::{Client, PostgrestPlugin};
use serde::Deserialize;
use serde_json::Value;

#[allow(dead_code)]
#[derive(Event, Debug, Deserialize)]
pub struct MyPostgresResponse {
    res: Value,
}

fn main() {
    let endpoint = "http://127.0.0.1:54321/rest/v1".into();

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
                send_every_second
                    .run_if(on_timer(Duration::from_secs(1)))
                    .run_if(is_logged_in),
                postgres_recv,
                postgres_err,
            ),
        )
        .register_request_type::<MyPostgresResponse>();

    app.run()
}

fn setup(mut commands: Commands, auth: Res<AuthClient>) {
    auth.sign_in(
        &mut commands,
        AuthCreds {
            id: "test@example.com".into(),
            password: "password".into(),
        },
    );
}

fn send_every_second(
    client: Res<Client>,
    mut evw: EventWriter<TypedRequest<MyPostgresResponse>>,
    auth: Option<Res<AuthClient>>,
) {
    let mut req = client.from("todos").select("*");

    if let Some(auth) = auth {
        if let Some(token) = auth.access_token.clone() {
            req = req.auth(token);
        }
    }

    let req = req.build();

    let req = HttpClient::new()
        .request(req)
        .with_type::<MyPostgresResponse>();

    evw.send(req);
}

fn postgres_recv(mut evr: EventReader<TypedResponse<MyPostgresResponse>>) {
    for ev in evr.read() {
        println!("[RECV] {:?}", ev);
    }
}

fn postgres_err(mut evr: EventReader<TypedResponseError<MyPostgresResponse>>) {
    for ev in evr.read() {
        println!("[ERR] {:?}", ev);
    }
}
