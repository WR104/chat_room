use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use std::sync::Arc;
use std::time::Instant;
use tokio_tungstenite::{accept_async, WebSocketStream};
use log::LevelFilter;
use rand::Rng;

const SERVER_ADDRESS: &str = "127.0.0.1:8080";
const MESSAGE_MAX_CAPACITY: usize = 5000;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::new()
        .filter_level(LevelFilter::Info)
        .init();
    let listener = tokio::net::TcpListener::bind(SERVER_ADDRESS).await?;
    log::info!("Listening on: {}", SERVER_ADDRESS);

    let (new_msg_notify_sender, _) = tokio::sync::broadcast::channel::<ChatMessage>(MESSAGE_MAX_CAPACITY);
    let app_state = Arc::new(AppState {
        new_msg_notify_sender,
        chat_history: tokio::sync::RwLock::new(Vec::new()),
    });
    while let Ok((stream, _addr)) = listener.accept().await {
        tokio::spawn(accept_connection(stream, app_state.clone()));
    }
    Ok(())
}

async fn accept_connection(
    stream: tokio::net::TcpStream,
    app_state: Arc<AppState>,
) -> tungstenite::Result<()> {
    let (sender, mut ws_receiver) = accept_async(stream).await.unwrap().split();
    let mut new_msg_notify_receiver = app_state.new_msg_notify_sender.subscribe();
    let mut connection_state = ConnectionState {
        connection_id: gen_random_user_id(),
        sender,
        new_msg_notify_sender: app_state.new_msg_notify_sender.clone(),
        last_heartbeat_timestamp: Instant::now(),
    };

    // send history chat messages when client connect
    for old_chat_msg in &*app_state.chat_history.read().await {
        connection_state
            .sender
            .send(tungstenite::Message::Text(old_chat_msg.to_string()))
            .await?;
    }
    // Send a notification to all other clients that a new client has joined
    let msg_new_client = String::from("joined in the room");
    connection_state.handle_client_msg(msg_new_client, &app_state.chat_history).await?;

    let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
    loop {
        tokio::select! {
            msg = ws_receiver.next() => {
                match msg {
                    Some(msg) => {
                        let msg = msg?;
                        match msg {
                            //clients send text message
                            tungstenite::Message::Text(msg) => connection_state.handle_client_msg(msg, &app_state.chat_history).await?,
                            tungstenite::Message::Ping(_) => {
                                connection_state.sender.send(tungstenite::Message::Pong(Vec::new())).await?;
                            },
                            //no message received
                            tungstenite::Message::Pong(_) => {
                                connection_state.last_heartbeat_timestamp = Instant::now();
                            },
                            tungstenite::Message::Close(_) => break,
                            tungstenite::Message::Binary(_) => unreachable!()
                        }
                    }
                    None => {
                        break
                    },
                }
            }
            // receive new msg notify
            Ok(msg) = new_msg_notify_receiver.recv() => {
                connection_state.sender.send(tungstenite::Message::Text(msg.to_string())).await?;
            }

            // send ping frame to client every HEARTBEAT_TIMEOUT second to keep alive and check dead client connection
            _ = interval.tick() => {
                if connection_state.last_heartbeat_timestamp.elapsed().as_secs() > ConnectionState::HEARTBEAT_TIMEOUT {
                    log::info!("server close dead connection");
                    break;
                }
                connection_state.sender.send(tungstenite::Message::Ping(Vec::new())).await?;
            }
        }
    }
    // Send a notification to all other clients that a client has left.
    let msg_client_left = String::from("left the room");
    connection_state.handle_client_msg(msg_client_left.clone(), &app_state.chat_history).await?;
    connection_state.sender.send(tungstenite::Message::Text(msg_client_left)).await?;

    Ok(())
}

#[derive(Clone, Debug)]
struct ChatMessage {
    created_at: chrono::NaiveDateTime,
    user_id: i32,
    message: String,
}

impl ToString for ChatMessage {
    fn to_string(&self) -> String {
        format!(
            "[user_id:{} {}]: {}",
            self.user_id,
            self.created_at.format("%H:%M:%S"),
            self.message
        )
    }
}

struct AppState {
    new_msg_notify_sender: tokio::sync::broadcast::Sender<ChatMessage>,
    chat_history: tokio::sync::RwLock<Vec<ChatMessage>>,
}

struct ConnectionState {
    connection_id: i32,
    // client last pong msg timestamp
    last_heartbeat_timestamp: Instant,
    sender: SplitSink<WebSocketStream<tokio::net::TcpStream>, tungstenite::Message>,
    new_msg_notify_sender: tokio::sync::broadcast::Sender<ChatMessage>,
}

impl ConnectionState {
    const HEARTBEAT_TIMEOUT: u64 = 15;
    async fn handle_client_msg(
        &mut self,
        msg: String,
        chat_history: &tokio::sync::RwLock<Vec<ChatMessage>>,
    ) -> tungstenite::Result<()> {
        let chat_message = ChatMessage {
            created_at: chrono::Local::now().naive_utc(),
            user_id: self.connection_id,
            message: msg.clone(),
        };
        self.new_msg_notify_sender
            .send(chat_message.clone())
            .unwrap();
        chat_history.write().await.push(chat_message);
        Ok(())
    }
}

fn gen_random_user_id() -> i32 {
    let mut rng = rand::thread_rng();
    let random_num = rng.gen_range(1000..10000);
    random_num
}