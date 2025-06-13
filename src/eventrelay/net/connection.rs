use std::sync::Arc;
use crate::error::{ErrorCode, ReplicashError};
use bytes::{Bytes, BytesMut};
use log::{debug, error, info, warn};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use uuid::Uuid;
use crate::eventrelay::net::events::{handle_event, handle_subscribe, handle_sync_ack, handle_sync_init, handle_unsubscribe};
use crate::eventrelay::handler::EventHandler;
use crate::tlv::message::TLVMessage;
use crate::tlv::types::EventType;
use tokio_util::sync::CancellationToken;

use std::sync::atomic::{AtomicBool, Ordering};

pub struct Client {
    id: Bytes,
    write_tx: UnboundedSender<TLVMessage>,
    cancel_token: CancellationToken,
    is_peer: AtomicBool,
    handler: Arc<EventHandler>,
}

impl Client {
    pub fn new(stream: TcpStream, handler: Arc<EventHandler>) -> Arc<Self> {
        let (reader, writer) = stream.into_split();
        let (write_tx, read_rx) = unbounded_channel::<TLVMessage>();

        let client = Arc::new(Self {
            id: Bytes::copy_from_slice(Uuid::new_v4().as_bytes()),
            write_tx,
            cancel_token: CancellationToken::new(),
            is_peer: AtomicBool::new(false),
            handler: handler.clone(),
        });

        // Subscribe to public channels
        handler.handle_subscribe_public(client.id());

        // Start read and write tasks
        let client_read = client.clone();
        let handler_read = handler.clone();
        tokio::spawn(async move {
            if let Err(e) = Client::read_task(client_read, reader, handler_read).await {
                warn!("Read task error: {:?}", e);
            }
        });

        let client_write = client.clone();
        tokio::spawn(async move {
            if let Err(e) = Client::write_task(writer, read_rx).await {
                warn!("Write task error: {:?}", e);
            }
        });

        client
    }


    async fn read_task(client: Arc<Client>, reader: OwnedReadHalf, handler: Arc<EventHandler>) -> Result<(), ReplicashError> {
        tokio::select! {
            _ = client.cancel_token.cancelled() => Ok(()),
            res = Client::read(client.clone(), reader, handler) => res,
        }
    }

    async fn read(client: Arc<Client>, mut reader: OwnedReadHalf, handler: Arc<EventHandler>) -> Result<(), ReplicashError> {
        debug!("Starting read loop for client {:?}", client.id);
        loop {
            debug!("Waiting for message from client {:?}", client.id);
            let mut len_buf = [0u8; 4];
            match reader.read_exact(&mut len_buf).await {
                Ok(_) => {
                    let len = u32::from_be_bytes(len_buf) as usize;
                    debug!("Received message header, length: {} bytes", len);

                    let mut full = BytesMut::with_capacity(len );
                    full.extend_from_slice(&len_buf);
                    full.resize(len , 0);

                    match reader.read_exact(&mut full[4..]).await {
                        Ok(_) => {
                            debug!("Received complete message of {} bytes", full.len());
                            let raw = full.freeze();

                            match TLVMessage::extract_event(&raw) {
                                Ok(event) => {
                                    info!("Processing event type: {:?} from client {:?}", event, client.id);

                                    match event {
                                        EventType::SyncInit => {
                                            debug!("Handling SyncInit event");
                                            handle_sync_init(handler.clone(), client.clone(), &raw).await
                                        },
                                        EventType::SyncAck => {
                                            debug!("Handling SyncAck event");
                                            handle_sync_ack(handler.clone(), client.clone(), &raw).await
                                        },
                                        EventType::Subscribe => {
                                            debug!("Handling Subscribe event");
                                            handle_subscribe(handler.clone(), client.clone(), &raw).await
                                        },
                                        EventType::Unsubscribe => {
                                            debug!("Handling Unsubscribe event");
                                            handle_unsubscribe(handler.clone(), client.clone(), &raw).await
                                        },
                                        EventType::Event => {
                                            debug!("Handling Event event");
                                            handle_event(handler.clone(), client.clone(), &raw).await
                                        },
                                        _ => {
                                            error!("Unknown event type: {:?}", event);
                                            return Err(ReplicashError::new(ErrorCode::InvalidEventType, "Unknown EventType"))
                                        },
                                    }
                                },
                                Err(e) => {
                                    error!("Failed to extract event type: {}", e);
                                    return Err(e);
                                }
                            }
                        },
                        Err(e) => {
                            error!("Failed to read message body: {}", e);
                            return Err(ReplicashError::new(ErrorCode::ReadFailed, "Connection failed"));
                        }
                    }
                },
                Err(e) => {
                    error!("Failed to read message header: {}", e);
                    return Err(ReplicashError::new(ErrorCode::ReadFailed, "Connection failed"));
                }
            }
        }
    }

    pub fn send(&self, tlv: &TLVMessage) -> Result<(), ReplicashError> {
        debug!("Sending message with event type {:?} to client {:?}", tlv.event_type, self.id);
        match self.write_tx.send(tlv.clone()) {
            Ok(_) => {
                debug!("Message sent successfully to client {:?}", self.id);
                Ok(())
            },
            Err(e) => {
                error!("Failed to send message to client {:?}: {}", self.id, e);
                Err(ReplicashError::new(ErrorCode::InternalServerError, "Failed to send message"))
            }
        }
    }

    // Alias for send method to maintain compatibility with existing code
    pub fn send_tlv(&self, tlv: &TLVMessage) -> Result<(), ReplicashError> {
        debug!("Using send_tlv alias for client {:?}", self.id);
        self.send(tlv)
    }

    async fn write_task(mut writer: OwnedWriteHalf, mut rx: UnboundedReceiver<TLVMessage>) -> Result<(), ReplicashError> {
        debug!("Starting write task");
        while let Some(data) = rx.recv().await {
            debug!("Writing message with event type {:?}", data.event_type);
            let encoded = data.encode();
            match writer.write_all(&encoded).await {
                Ok(_) => {
                    debug!("Successfully wrote {} bytes", encoded.len());
                },
                Err(e) => {
                    error!("Failed to write message: {}", e);
                    return Err(ReplicashError::new(ErrorCode::WriteFailed, "Connection failed"));
                }
            }
        }
        info!("Write task completed");
        Ok(())
    }

    pub fn is_peer(&self) -> bool {
        let is_peer = self.is_peer.load(Ordering::Relaxed);
        debug!("Checking if client {:?} is peer: {}", self.id, is_peer);
        is_peer
    }

    pub fn set_peer(&self, is_peer: bool) {
        debug!("Setting client {:?} peer status to: {}", self.id, is_peer);
        self.is_peer.store(is_peer, Ordering::Relaxed);
    }

    pub fn close(&self) {
        info!("Closing connection for client {:?}", self.id);
        // Close the connection with a proper TLVMessage
        let close_msg = TLVMessage::new(EventType::Ok);
        debug!("Sending close message to client {:?}", self.id);
        match self.write_tx.send(close_msg) {
            Ok(_) => debug!("Close message sent successfully"),
            Err(e) => warn!("Failed to send close message: {}", e),
        }

        // Cancel the read task
        debug!("Cancelling read task for client {:?}", self.id);
        self.cancel_token.cancel();
        info!("Connection closed for client {:?}", self.id);
    }

    pub fn id(&self) -> Bytes {
        self.id.clone()
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        info!("Client {:?} dropped, cleaning up resources", self.id);

        // Close the connection if it's not already closed
        self.close();

        // Clean up all subscriptions and unregister from the connection registry
        self.handler.handle_client_disconnect(self.id.clone());

        info!("Client {:?} cleanup completed", self.id);
    }
}
