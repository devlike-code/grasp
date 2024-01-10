use std::collections::HashMap;
use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::{foundation::GraspEditorState, windows::GraspEditorWindow};
use futures::{SinkExt, StreamExt};
use mosaic::internals::Mosaic;
use tokio::time::sleep;
use warp::filters::ws::Message;
use warp::{filters::ws::WebSocket, Filter};

pub static ID: AtomicUsize = AtomicUsize::new(0);
use lazy_static::lazy_static;

lazy_static! {
    pub static ref DOTS: Arc<Mutex<HashMap<usize, String>>> = Arc::new(Mutex::new(HashMap::new()));
}

pub async fn client_connection(ws: WebSocket, id: u16) {
    println!("establishing client connection... {:?}", ws);

    let (mut sender, mut _receiver) = ws.split();

    let mut last_message = String::new();

    loop {
        let maybe_message = {
            let lock = DOTS.lock().unwrap();
            lock.get(&(id as usize)).cloned()
        };

        if let Some(message) = maybe_message {
            if last_message != message {
                last_message = message.clone();
                sender.send(Message::text(message)).await.unwrap();
            }
        }

        sleep(Duration::from_millis(1000)).await;
    }
}

pub async fn run_server(id: u16) {
    println!("Running server for id {:?}", id);
    let websocket = warp::path("graph")
        .and(warp::ws())
        .map(move |ws: warp::ws::Ws| {
            println!("Upgrading connection to websocket");
            ws.on_upgrade(move |ws| client_connection(ws, id))
        });

    warp::serve(websocket.with(warp::cors().allow_any_origin()))
        .run(([127, 0, 0, 1], id))
        .await;
}

pub const MIN_PORT: usize = 9001;

pub trait Networked {
    fn get_id(&self) -> usize;
    fn prepare_content(&self) -> String;

    fn initialize_networked(&mut self) {
        println!("Registering new id: {}", self.get_id());

        let id = self.get_id() as u16;
        tokio::spawn(run_server(id));
    }
}

impl Networked for GraspEditorState {
    fn get_id(&self) -> usize {
        MIN_PORT + self.editor_state_tile.id
    }

    fn prepare_content(&self) -> String {
        self.editor_mosaic.dot("Editor")
    }
}

impl Networked for Arc<Mosaic> {
    fn get_id(&self) -> usize {
        MIN_PORT + self.id
    }

    fn prepare_content(&self) -> String {
        self.dot(format!("Window_{}", self.id).as_str())
    }
}
