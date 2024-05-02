use bevy::prelude::*;
use bevy_gotrue::{just_logged_in, AuthPlugin, Client};
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

fn do_login(client: Res<Client>) {
    let g = client.get_url_for_provider("google");
    println!("\n[LOGIN]\nGo to this URL to sign in: \n{}\n", g);
}

fn did_login(client: Res<Client>) {
    println!("Login complete. {:?}", client.access_token);
}
