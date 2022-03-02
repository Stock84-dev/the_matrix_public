use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::Arc;

use futures_util::stream::{SplitSink, SplitStream};
use futures_util::StreamExt;
use merovingian::Readable;
use mouse::log::*;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message as TMessage;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use crate::zion::ZionCommand;
use crate::Command;

pub struct Listener {
    inner: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, TMessage>,
}

impl Listener {
    pub fn listen(
        addr: SocketAddr,
        commands: Arc<Mutex<VecDeque<ZionCommand>>>,
        stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    ) -> Listener {
        let (sink, stream) = stream.split();
        tokio::spawn(listen(addr, stream, commands));
        Listener { inner: sink }
    }

    // pub async fn send(&self, message: Message) ->Result<()> {
    //     self.inner.send(TMessage::)
    //     Ok(())
    // }
}

async fn listen(
    addr: SocketAddr,
    mut stream: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    commands: Arc<Mutex<VecDeque<ZionCommand>>>,
) {
    while let Some(msg) = stream.next().await {
        match msg {
            Ok(msg) => match msg {
                TMessage::Text(_) => {}
                TMessage::Binary(data) => match Command::read_from_buffer(&data) {
                    Ok(command) => {
                        commands.lock().await.push_back(ZionCommand {
                            addr: addr.clone(),
                            command: Some(command),
                        });
                    }
                    Err(e) => error!("Failed to interpret message. {:#?}", e),
                },
                TMessage::Ping(_) => {}
                TMessage::Pong(_) => {}
                TMessage::Close(_) => break,
            },
            Err(e) => {
                error!("Websocket error, closing connection: {:#?}", e);
                break;
            }
        }
    }
    info!("Connection closed for {}", addr);
    commands.lock().await.push_back(ZionCommand {
        addr: addr.clone(),
        command: None,
    });
}

// pub async fn communicate(tower: Arc<Tower>) {
//     let addr = "127.0.0.1:8080".to_string();
//     let try_socket = TcpListener::bind(&addr).await;
//     let mut listener = try_socket.expect("Failed to bind");
//     while let Ok((stream, addr)) = listener.accept().await {
//         tokio::spawn(handle_connection_with_errors(tower.clone(), stream, addr));
//     }
// }
//
// async fn handle_connection_with_errors(tower: Arc<Tower>, raw_stream: TcpStream, addr: SocketAddr) {
//     trace!("Incoming TCP connection from: {}", addr);
//     let ws_stream = tokio_tungstenite::accept_async(MaybeTlsStream::Plain(raw_stream))
//         .await
//         .expect("Error during the websocket handshake occurred");
//     let command_ws = CommandWs::from(ws_stream);
//     let (sender, receiver) = command_ws.split();
//     let mut connections = tower.open_connections.write().await;
//     let id = connections.len() + 1;
//     connections.insert(
//         addr.clone(),
//         Mutex::new(Connection {
//             id,
//             matrix_id: None,
//             sender,
//         }),
//     );
//     drop(connections);
//
//     if let Err(e) = handle_connection(tower.deref(), receiver, addr).await {
//         error!("{:#?}", e);
//     }
//     close_connection(tower.deref(), addr).await;
// }
//
// async fn close_connection(tower: &Tower, addr: SocketAddr) {
//     let mut connections = tower.open_connections.write().await;
//     let mut connection = connections.remove(&addr).unwrap().into_inner();
//     if let Err(e) = connection.sender.close().await {
//         error!("{:#?}", e);
//     }
// }
//
// async fn handle_connection(
//     tower: &Tower,
//     mut receiver: SplitStream<CommandWs>,
//     addr: SocketAddr,
// ) -> Result<()> {
//     info!("WebSocket connection established: {}", addr);
//     let mut last_heart_beat_ts = Utc::now().timestamp();
//     const HEART_BEAT_INTERVAL: i64 = 120;
//
//     loop {
//         let until = last_heart_beat_ts + HEART_BEAT_INTERVAL - Utc::now().timestamp();
//         if until < 0 {
//             error!("Client died!");
//             break;
//         }
//         let mut heart_beat = time::delay_for(Duration::from_secs(until as u64));
//
//         if let Err(_) = tokio::select! {
//             _ = &mut heart_beat => {
//                 error!("Client died!");
//                 Err(())
//             },
//             response = handle_response(tower, &addr, &mut receiver, &mut last_heart_beat_ts) => {
//                 response.map_err(|x| ())
//             }
//         } {
//             trace!("Connection closed.");
//             break;
//         }
//     }
//     Ok(())
// }
//
// async fn handle_response(
//     tower: &Tower,
//     addr: &SocketAddr,
//     receiver: &mut SplitStream<CommandWs>,
//     last_heart_beat_ts: &mut i64,
// ) -> Result<()> {
//     if let Some(msg) = receiver.next().await {
//         let response: Response = msg?;
//         match response {
//             Response::HeartBeat => {
//                 *last_heart_beat_ts = Utc::now().timestamp();
//             }
//             Response::Authentication(matrix_id) => {
//                 let connections = tower.open_connections.read().await;
//                 let connection = connections.get(addr).unwrap();
//                 info!("Authenticated: {:#?}.", matrix_id);
//                 connection.lock().await.matrix_id = Some(matrix_id);
//             }
//         }
//     } else {
//         return Err(anyhow::anyhow!("connection closed"));
//     }
//     Ok(())
// }
