// Authed todo list
// almost obligatory for a backend system lmao
//

use std::time::Duration;

use bevy::{ecs::system::SystemId, prelude::*};
use bevy_cosmic_edit::*;
use bevy_gotrue::just_logged_in;
use bevy_http_client::prelude::HttpTypedRequestTrait;
use bevy_http_client::prelude::TypedRequest;
use bevy_http_client::prelude::TypedResponse;
use bevy_http_client::HttpClient;
use bevy_http_client::HttpClientPlugin;
use bevy_supabase::AuthClient;
use bevy_supabase::PostgrestClient;
use bevy_supabase::SupabasePlugin;
use chrono::DateTime;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Component)]
struct LoginMarker;

#[derive(Component)]
struct MainScreenMarker;

#[derive(Component)]
struct TaskContainer;

#[derive(Event, Debug, Deserialize)]
pub struct TodoTaskList(Vec<TodoTask>);

#[derive(Debug, Deserialize, Clone)]
struct TodoTask {
    id: u8,
    inserted_at: DateTime<chrono::Local>,
    is_complete: bool,
    task: String,
    user_id: Uuid,
}

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
            apikey: std::env::var("SUPABASE_LOCAL_ANON_KEY").unwrap().into(),
            endpoint: "http://127.0.0.1:54321".into(),
            ..default()
        },
        CosmicEditPlugin::default(),
    ))
    .register_request_type::<TodoTaskList>()
    .add_systems(Startup, (setup, build_login_screen).chain())
    .add_systems(
        Update,
        (
            ((
                change_active_editor_ui,
                tick_debounce_timers,
                trigger_buttons_on_click,
                read_incoming_task_list,
                update_edited_task,
                apply_deferred,
                evaluate_callbacks,
            )
                .chain()
                .after(FocusSet),),
            (login_complete, build_main_screen, get_list).run_if(just_logged_in),
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

    let todo_task_callback = world.register_system(todo_task_callback);
    world.insert_resource(TodoTaskCallback(todo_task_callback));
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
    auth: Res<AuthClient>,
) {
    let email = email.single();
    let password = password.single();

    println!("{} | {}", email.get_text(), password.get_text());

    commands.run_system_with_input(
        auth.sign_in,
        bevy_gotrue::AuthCreds {
            password: password.get_text(),
            id: email.get_text(),
        },
    );
}

fn login_complete(mut commands: Commands, login: Query<Entity, With<LoginMarker>>) {
    let login = login.single();
    commands.entity(login).despawn_recursive();

    println!("Login complete!");
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
                        .with_text(&mut font_system, "test@example.com", attrs),
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
                        .with_text(&mut font_system, "password", attrs),
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
        .insert(LoginMarker)
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
            .insert(Debouncer::default());
        });
}

fn build_main_screen(mut commands: Commands) {
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
        .insert(MainScreenMarker)
        .with_children(|cb| {
            cb.spawn(
                TextBundle {
                    text: Text::from_section(
                        "ToDo",
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

            cb.spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(90.0),
                    flex_wrap: FlexWrap::Wrap,
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Start,
                    row_gap: Val::Px(5.0),
                    ..default()
                },
                background_color: BackgroundColor(Color::WHITE.with_a(0.2)),
                ..default()
            })
            .insert(TaskContainer);
        });
}

fn get_list(client: Res<PostgrestClient>, mut evw: EventWriter<TypedRequest<TodoTaskList>>) {
    let req = client.from("todos").select("*").build();

    let req = HttpClient::new().request(req).with_type::<TodoTaskList>();

    evw.send(req);
}

fn read_incoming_task_list(
    mut commands: Commands,
    mut evr: EventReader<TypedResponse<TodoTaskList>>,
    todo_task_callback: Res<TodoTaskCallback>,
) {
    for ev in evr.read() {
        for task in &ev.0 {
            commands.run_system_with_input(todo_task_callback.0, task.clone());
        }
    }
}

#[derive(Resource)]
struct TodoTaskCallback(pub SystemId<TodoTask>);

#[derive(Component)]
struct TaskId(pub u8);

fn todo_task_callback(
    In(task): In<TodoTask>,
    mut commands: Commands,
    container: Query<Entity, With<TaskContainer>>,
    mut font_system: ResMut<CosmicFontSystem>,
) {
    println!(
        "[TASK] {} {} {} {} {}",
        task.id, task.task, task.is_complete, task.inserted_at, task.user_id
    );

    let attrs = Attrs::new();

    let editor =
        commands
            .spawn((
                CosmicEditBundle {
                    buffer: CosmicBuffer::new(&mut font_system, Metrics::new(20.0, 20.0))
                        .with_text(&mut font_system, task.task.as_str(), attrs),
                    max_lines: MaxLines(1),
                    default_attrs: DefaultAttrs(AttrsOwned::new(attrs)),
                    ..default()
                },
                Placeholder::new("Task", attrs.color(Color::GRAY.to_cosmic())),
                TaskId(task.id),
            ))
            .id();

    let e = container.single();

    commands.entity(e).with_children(move |cb| {
        cb.spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Px(40.0),
                ..default()
            },
            background_color: BackgroundColor(Color::BLUE.with_a(0.2)),
            ..default()
        })
        .with_children(move |cb| {
            cb.spawn(ButtonBundle {
                style: Style {
                    width: Val::Percent(80.),
                    height: Val::Percent(100.),
                    ..default()
                },
                background_color: BackgroundColor(Color::WHITE),
                ..default()
            })
            .insert(CosmicSource(editor));
        });
    });
}

fn update_edited_task(
    editor_q: Query<(&CosmicBuffer, &TaskId)>,
    mut evr: EventReader<CosmicTextChanged>,
) {
    for ev in evr.read() {
        if let Ok((editor, id)) = editor_q.get(ev.0 .0) {
            println!("ID: {}", id.0);
            println!("TEXT: {}", ev.0 .1);
            println!("GETTEXT: {}", editor.get_text());
        }
    }
}
