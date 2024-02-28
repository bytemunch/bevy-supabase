pub mod api;
pub mod builder;
pub mod filter;

use std::marker::PhantomData;

use api::Postgrest;
use bevy::prelude::*;
use builder::Builder;

pub trait AppExtend {
    fn add_postgrest_event<T: Event + ResponseEvent>(&mut self) -> &mut Self;
}

impl AppExtend for App {
    fn add_postgrest_event<T: Event + ResponseEvent>(&mut self) -> &mut Self {
        self.add_event::<T>()
            .add_event::<PostgresRequest<T>>()
            .add_systems(Update, (send_postgres_requests::<T>,))
    }
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

impl PostgrestPlugin {
    pub fn new(endpoint: String) -> Self {
        Self { endpoint }
    }
}

impl Plugin for PostgrestPlugin {
    fn build(&self, app: &mut App) {
        // TODO build + add api resource
        app.insert_resource(Postgrest::new(self.endpoint.clone()))
            .add_systems(Update, (send_postgres_requests::<PostgresResponse>,))
            .add_event::<PostgresResponse>()
            .add_event::<PostgresRequest<PostgresResponse>>();
    }
}

pub fn send_request<T: Event + ResponseEvent>(In(req): In<Builder>, mut evw: EventWriter<T>) {
    match req.execute() {
        Ok(res) => {
            evw.send(T::new(
                res.text().expect("Bro tried to string im dead").into(),
            ));
        }
        Err(e) => {
            println!("Response is error: {:?}", e)
        }
    }
}

fn send_postgres_requests<T: Event + ResponseEvent>(
    mut evr: EventReader<PostgresRequest<T>>,
    mut evw: EventWriter<T>,
) {
    for ev in evr.read() {
        match ev.req.clone().execute() {
            Ok(res) => {
                evw.send(T::new(
                    res.text().expect("Bro tried to string im dead").into(),
                ));
            }
            Err(e) => {
                println!("Response is error: {:?}", e)
            }
        }
    }
}
