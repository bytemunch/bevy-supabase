use bevy::prelude::*;
use bevy_gotrue::{just_logged_in, AuthCreds, AuthPlugin, Client};
use bevy_http_client::HttpClientPlugin;

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins)
        .add_plugins(HttpClientPlugin)
        .add_plugins(AuthPlugin {
            endpoint: "http://localhost:54321/auth/v1".into(),
        })
        .add_systems(Startup, do_login)
        .add_systems(Update, did_login.run_if(just_logged_in))
        .run()
}

fn do_login(mut commands: Commands, client: Res<Client>) {
    let creds = AuthCreds {
        id: "test@example.com".into(),
        password: "password".into(),
    };
    commands.run_system_with_input(client.sign_in, creds);
}

fn did_login(client: Res<Client>) {
    println!("Login complete. {:?}", client);
}
