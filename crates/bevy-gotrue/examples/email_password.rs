use bevy::prelude::*;
use bevy_gotrue::{AuthClient, AuthPlugin};

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins)
        .add_plugins(AuthPlugin {
            endpoint: "http://172.18.0.4:9999".into(),
        })
        .add_systems(Startup, do_login)
        .run()
}

fn do_login(mut commands: Commands, mut auth: ResMut<AuthClient>) {
    match auth.sign_in(
        &mut commands,
        go_true::EmailOrPhone::Email("test@example.com".into()),
        "password".into(),
    ) {
        Ok(sesh) => println!("Got sesh! {:?}", sesh),
        Err(e) => println!("Ne sesh! {:?}", e),
    }
}
