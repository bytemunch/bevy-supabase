use std::marker::PhantomData;

use bevy::prelude::*;
use postgrest::*;

pub trait AppExtend {
    fn add_postgrest_event<T: Event + ResponseEvent>(&mut self) -> &mut Self;
}

impl AppExtend for App {
    fn add_postgrest_event<T: Event + ResponseEvent>(&mut self) -> &mut Self {
        self.add_event::<T>()
            .add_event::<PostgresRequest<T>>()
            .add_systems(
                Update,
                (send_postgres_requests::<T>, read_postgres_responses::<T>),
            )
    }
}

#[derive(Resource)]
pub struct Client {
    pub client: Postgrest,
    rt: tokio::runtime::Runtime,
}

#[derive(Event)]
pub struct PostgresRequest<T: Event + ResponseEvent> {
    pub req: Builder,
    spoopy: PhantomData<T>,
}

impl<T: Event + ResponseEvent> PostgresRequest<T> {
    pub fn new(req: Builder) -> Self {
        Self {
            req,
            spoopy: PhantomData::<T>,
        }
    }
}

pub trait ResponseEvent {
    fn new(res: String) -> Self;
    fn get_res(&self) -> String;
}

#[derive(Event, Debug)]
pub struct PostgresResponse {
    res: String,
}

impl ResponseEvent for PostgresResponse {
    fn new(res: String) -> Self {
        Self { res }
    }

    fn get_res(&self) -> String {
        self.res.clone()
    }
}

pub struct PostgrestPlugin {
    pub endpoint: String,
}

impl Plugin for PostgrestPlugin {
    fn build(&self, app: &mut App) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let client = Postgrest::new(self.endpoint.clone());

        app.insert_resource(Client { client, rt })
            .add_systems(
                Update,
                (
                    send_postgres_requests::<PostgresResponse>,
                    read_postgres_responses::<PostgresResponse>,
                ),
            )
            .add_event::<PostgresResponse>()
            .add_event::<PostgresRequest<PostgresResponse>>();
    }
}

fn send_postgres_requests<T: Event + ResponseEvent>(
    mut evr: EventReader<PostgresRequest<T>>,
    client: Res<Client>,
    mut evw: EventWriter<T>,
) {
    client.rt.block_on(async move {
        for ev in evr.read() {
            match ev.req.clone().execute().await {
                Ok(res) => {
                    evw.send(T::new(
                        res.text().await.expect("Bro tried to string im dead"),
                    ));
                }
                Err(e) => {
                    println!("Response is error: {:?}", e)
                }
            }
        }
    });
}

fn read_postgres_responses<T: ResponseEvent + Event>(mut evr: EventReader<T>) {
    for ev in evr.read() {
        println!("{:?}", ev.get_res());
    }
}
