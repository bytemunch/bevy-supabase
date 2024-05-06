// Authed todo list
// almost obligatory for a backend system lmao
//

use std::time::Duration;

use bevy::{ecs::system::SystemId, prelude::*};
use bevy_cosmic_edit::*;
use bevy_http_client::HttpClientPlugin;
use bevy_supabase::SupabasePlugin;

#[derive(Component)]
struct LoginMarker;

#[derive(Component)]
struct LoginButton;

#[derive(Component)]
struct Callback(SystemId);

#[derive(Component)]
struct Triggered;

#[derive(Component)]
struct Debouncer {
    pub timer: Timer,
    pub pressed: bool,
}

impl Default for Debouncer {
    fn default() -> Self {
        Self {
            timer: Timer::new(Duration::from_secs_f32(0.3), TimerMode::Once),
            pressed: false,
        }
    }
}

fn main() {
    let mut app = App::new();

    app.add_plugins((
        DefaultPlugins,
        HttpClientPlugin,
        SupabasePlugin {
            apikey: "NotNeededForLocal".into(),
            endpoint: "http://127.0.0.1:54321".into(),
            ..default()
        },
        CosmicEditPlugin::default(),
    ))
    .add_systems(Startup, (setup, build_login_screen).chain())
    .add_systems(
        Update,
        (
            (
                tick_debounce_timers,
                trigger_buttons_on_click,
                // apply_deferred,
                evaluate_callbacks,
            )
                .chain(),
            change_active_editor_ui,
        ),
    );

    app.run()
}

#[derive(Resource)]
struct SystemManifest {
    do_login: SystemId,
}

fn setup(world: &mut World) {
    let do_login = world.register_system(do_login);
    let manifest = SystemManifest { do_login };

    world.insert_resource(manifest);
}

fn evaluate_callbacks(query: Query<(Entity, &Callback), With<Triggered>>, mut commands: Commands) {
    for (e, callback) in query.iter() {
        commands.entity(e).remove::<Triggered>();
        commands.run_system(callback.0);
    }
}

fn tick_debounce_timers(mut q: Query<&mut Debouncer>, time: Res<Time>) {
    for mut t in q.iter_mut() {
        t.timer.tick(time.delta());
    }
}

fn trigger_buttons_on_click(
    mut commands: Commands,
    mut q: Query<
        (Entity, &Interaction, Option<&mut Debouncer>),
        (With<Button>, With<Callback>, Without<Triggered>),
    >,
) {
    for (e, i, t) in q.iter_mut() {
        match i {
            Interaction::Pressed => {
                if let Some(mut t) = t {
                    if t.pressed {
                        t.timer.reset();
                        continue;
                    }

                    if !t.timer.finished() {
                        continue;
                    }

                    t.timer.reset();
                    t.pressed = true;
                }
                commands.entity(e).insert(Triggered);
            }
            _ => {
                if let Some(mut t) = t {
                    t.pressed = false;
                }
            }
        }
    }
}

#[derive(Component)]
struct EmailBuffer;

#[derive(Component)]
struct PasswordBuffer;

fn do_login(
    mut commands: Commands,
    email: Query<&CosmicBuffer, With<EmailBuffer>>,
    password: Query<&CosmicBuffer, With<PasswordBuffer>>,
    login: Query<Entity, With<LoginMarker>>,
) {
    println!("DO LOGIN POGGIES");
    // let email = email.single();
    // let password = password.single();
    // let login = login.single();
    //
    // commands.entity(login).despawn_recursive();
}

fn build_login_screen(
    mut commands: Commands,
    mut font_system: ResMut<CosmicFontSystem>,
    manifest: Res<SystemManifest>,
) {
    commands.spawn(Camera2dBundle { ..default() });

    let attrs = Attrs::new();

    let email_editor =
        commands
            .spawn((
                CosmicEditBundle {
                    buffer: CosmicBuffer::new(&mut font_system, Metrics::new(32.0, 32.0))
                        .with_text(&mut font_system, "", attrs),
                    max_lines: MaxLines(1),
                    default_attrs: DefaultAttrs(AttrsOwned::new(attrs)),
                    ..default()
                },
                Placeholder::new("Email", attrs.color(Color::GRAY.to_cosmic())),
                EmailBuffer,
            ))
            .id();

    let password_editor =
        commands
            .spawn((
                CosmicEditBundle {
                    buffer: CosmicBuffer::new(&mut font_system, Metrics::new(32.0, 32.0))
                        .with_text(&mut font_system, "", attrs),
                    max_lines: MaxLines(1),
                    default_attrs: DefaultAttrs(AttrsOwned::new(attrs)),
                    ..default()
                },
                Placeholder::new("Password", attrs.color(Color::GRAY.to_cosmic())),
                Password::default(),
                PasswordBuffer,
            ))
            .id();

    let login_button_text = commands
        .spawn((
            CosmicEditBundle {
                buffer: CosmicBuffer::new(&mut font_system, Metrics::new(32.0, 32.0)).with_text(
                    &mut font_system,
                    "Sign In",
                    attrs.color(Color::WHITE.to_cosmic()),
                ),
                fill_color: CosmicBackgroundColor(Color::LIME_GREEN),
                hover_cursor: HoverCursor(CursorIcon::Pointer),
                ..default()
            },
            ReadOnly,
            UserSelectNone,
        ))
        .id();

    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Vw(90.0),
                height: Val::Vh(90.0),
                position_type: PositionType::Absolute,
                left: Val::Vw(5.0),
                top: Val::Vh(5.0),
                flex_wrap: FlexWrap::Wrap,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Start,
                row_gap: Val::Px(5.0),
                ..default()
            },
            background_color: BackgroundColor(Color::BLACK.with_a(0.2)),
            ..default()
        })
        .with_children(|cb| {
            cb.spawn(
                TextBundle {
                    text: Text::from_section(
                        "Login",
                        TextStyle {
                            font_size: 40.0,
                            ..default()
                        },
                    ),
                    ..default()
                }
                .with_style(Style {
                    height: Val::Px(40.),
                    margin: UiRect {
                        bottom: Val::Px(20.),
                        ..default()
                    },
                    ..default()
                })
                .with_text_justify(JustifyText::Center),
            );

            cb.spawn(ButtonBundle {
                style: Style {
                    width: Val::Percent(80.),
                    height: Val::Px(40.),
                    ..default()
                },
                background_color: BackgroundColor(Color::WHITE),
                ..default()
            })
            .insert(CosmicSource(email_editor));

            cb.spawn(ButtonBundle {
                style: Style {
                    width: Val::Percent(80.),
                    height: Val::Px(40.),
                    ..default()
                },
                background_color: BackgroundColor(Color::WHITE),
                ..default()
            })
            .insert(CosmicSource(password_editor));

            cb.spawn(ButtonBundle {
                style: Style {
                    width: Val::Px(200.0),
                    height: Val::Px(40.0),
                    ..default()
                },
                background_color: BackgroundColor(Color::WHITE),
                ..default()
            })
            .insert(CosmicSource(login_button_text))
            .insert(Callback(manifest.do_login))
            .insert(LoginButton)
            .insert(Debouncer::default());
        });
}
