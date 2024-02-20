use bevy::prelude::*;
pub use go_true::session::Session;
pub use go_true::EmailOrPhone;

#[derive(Debug)]
pub enum AuthError {
    CatchAllPleaseMakeMeMoreSpecific,
}

#[derive(Resource)]
pub struct AuthClient {
    api: go_true::Api,
    rt: tokio::runtime::Runtime,
}

#[derive(Resource)]
pub struct AuthSession(pub Session);

#[derive(Resource, Default)]
pub struct AuthCreds {
    id: Option<EmailOrPhone>,
    password: Option<String>,
}

impl AuthClient {
    pub fn sign_in(
        &mut self,
        commands: &mut Commands,
        id: EmailOrPhone,
        password: String,
    ) -> Result<Session, AuthError> {
        let sesh = self.rt.block_on(self.api.sign_in(id, password));

        match sesh {
            Ok(s) => {
                commands.insert_resource(AuthSession(s.clone()));
                Ok(s)
            }
            Err(_e) => Err(AuthError::CatchAllPleaseMakeMeMoreSpecific),
        }
    }
}

pub struct AuthPlugin {
    pub endpoint: String,
}

impl Plugin for AuthPlugin {
    fn build(&self, app: &mut App) {
        let api = go_true::Api::new(self.endpoint.clone());
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        app.insert_resource(AuthClient { api, rt })
            .init_resource::<AuthCreds>();
    }
}

pub fn sign_in(mut commands: Commands, auth_client: Res<AuthClient>, auth_creds: Res<AuthCreds>) {
    let Some(id) = auth_creds.id.clone() else {
        return;
    };
    let Some(password) = auth_creds.password.clone() else {
        return;
    };

    let Ok(sesh) = auth_client
        .rt
        .block_on(auth_client.api.sign_in(id, password))
    else {
        return;
    };

    commands.insert_resource(AuthSession(sesh));
}
