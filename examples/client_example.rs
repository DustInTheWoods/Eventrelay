
use tokio::net::TcpStream;
use tokio::io::{AsyncWriteExt, AsyncReadExt, BufReader, BufWriter};
use bytes::Bytes;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use std::io::{stdin, stdout, Write};
use std::sync::Arc;
use std::sync::Mutex;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use log::{info, warn, error, debug, trace, LevelFilter};
use env_logger::{self, Builder, Env};
use Eventrelay::tlv::message::TLVMessage;
use Eventrelay::tlv::types::{EventType, FieldType};

const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024;

#[tokio::main]
async fn main() {
    // Initialize the logger with enhanced formatting
    let env = Env::default().default_filter_or("info");
    Builder::from_env(env)
        .format_timestamp_millis()
        .format_module_path(true)
        .format_level(true)
        .format_target(false)
        .init();

    debug!("Client example logger initialized");

    let addr = "127.0.0.1";
    let port = prompt("Port: ");
    let full_addr = format!("{}:{}", addr, port);
    info!("Connecting to address: {}", full_addr);

    println!("Mode selection:");
    println!("1 = Ping");
    println!("2 = Subscribe + Chat");
    println!("3 = Event senden");
    println!("4 = Performance-Test");
    println!("5 = Subscribe + Listen");
    use std::time::Instant;

    let mode = prompt("Select mode (1-5): ");
    debug!("Selected mode: {}", mode.trim());

    match mode.trim() {
        "4" => {
            info!("Starting performance test");
            dual_client_performance_test(full_addr).await;
        },
        _ => {
            debug!("Attempting to connect to {}", full_addr);
            match TcpStream::connect(&full_addr).await {
                Ok(stream) => {
                    info!("Connection established successfully");
                    if let Err(e) = stream.set_nodelay(true) {
                        warn!("Failed to set TCP_NODELAY: {}", e);
                    } else {
                        debug!("TCP_NODELAY set successfully");
                    }

                    let (reader_raw, writer_raw) = stream.into_split();
                    let reader = BufReader::new(reader_raw);
                    let writer = BufWriter::new(writer_raw);

                    match mode.trim() {
                        "1" => {
                            info!("Starting ping mode");
                            send_ping(writer).await;
                        },
                        "2" => {
                            info!("Starting chat mode");
                            chat_mode(reader, writer).await;
                        },
                        "3" => {
                            info!("Starting custom event mode");
                            send_custom_event(writer).await;
                        },
                        "5" => {
                            info!("Starting subscribe and listen mode");
                            subscribe_listen(reader, writer).await;
                        },
                        _ => {
                            error!("Invalid mode selection: {}", mode.trim());
                            println!("Invalid selection");
                        },
                    }
                }
                Err(e) => {
                    error!("Connection failed: {}", e);
                    eprintln!("Connection failed: {}", e);
                }
            }
        }
    }
}

async fn dual_client_performance_test(addr: String) {
    info!("Starting performance test with dual clients");
    debug!("Performance test parameters: address={}, topic=perf/test, messages=1000", addr);

    let start = Instant::now();
    let topic = "perf/test";
    let messages = 1000;
    let counter = Arc::new(AtomicUsize::new(0));

    // Store send timestamps for each message
    let message_timestamps = Arc::new(Mutex::new(HashMap::<usize, u64>::new()));
    debug!("Initialized message timestamp tracking");

    // Connect receiver client
    debug!("Connecting receiver client to {}", addr);
    let stream_recv = match TcpStream::connect(&addr).await {
        Ok(stream) => {
            info!("Receiver client connected successfully");
            if let Err(e) = stream.set_nodelay(true) {
                warn!("Failed to set TCP_NODELAY for receiver: {}", e);
            }
            stream
        },
        Err(e) => {
            error!("Failed to connect receiver client: {}", e);
            eprintln!("Connection failed: {}", e);
            return;
        }
    };

    // Connect sender client
    debug!("Connecting sender client to {}", addr);
    let stream_send = match TcpStream::connect(&addr).await {
        Ok(stream) => {
            info!("Sender client connected successfully");
            if let Err(e) = stream.set_nodelay(true) {
                warn!("Failed to set TCP_NODELAY for sender: {}", e);
            }
            stream
        },
        Err(e) => {
            error!("Failed to connect sender client: {}", e);
            eprintln!("Connection failed: {}", e);
            return;
        }
    };

    let (recv_reader_raw, recv_writer_raw) = stream_recv.into_split();
    let mut recv_reader = BufReader::new(recv_reader_raw);
    let mut recv_writer = BufWriter::new(recv_writer_raw);

    let (_, send_writer_raw) = stream_send.into_split();
    let mut send_writer = BufWriter::new(send_writer_raw);

    // Create subscription message
    debug!("Creating subscription message for topic: {}", topic);
    let mut sub = TLVMessage::new(EventType::Subscribe);
    sub.insert_field(FieldType::Key, Bytes::from(topic));
    let encoded_sub = sub.encode();

    // Subscribe on the receive connection
    debug!("Subscribing receiver client to topic: {}", topic);
    match recv_writer.write_all(&encoded_sub).await {
        Ok(_) => {
            match recv_writer.flush().await {
                Ok(_) => info!("Receiver client subscribed to topic: {}", topic),
                Err(e) => {
                    error!("Failed to flush after subscribing receiver: {}", e);
                    eprintln!("Subscription error: {}", e);
                    return;
                }
            }
        },
        Err(e) => {
            error!("Failed to subscribe receiver client: {}", e);
            eprintln!("Subscription error: {}", e);
            return;
        }
    }

    // Subscribe on the send connection
    debug!("Subscribing sender client to topic: {}", topic);
    match send_writer.write_all(&encoded_sub).await {
        Ok(_) => {
            match send_writer.flush().await {
                Ok(_) => info!("Sender client subscribed to topic: {}", topic),
                Err(e) => {
                    error!("Failed to flush after subscribing sender: {}", e);
                    eprintln!("Subscription error: {}", e);
                    return;
                }
            }
        },
        Err(e) => {
            error!("Failed to subscribe sender client: {}", e);
            eprintln!("Subscription error: {}", e);
            return;
        }
    }

    let counter_clone = counter.clone();
    let message_timestamps_clone = message_timestamps.clone();
    let mut latenzen = Vec::with_capacity(messages);

    info!("Starting message receiver task");
    let handle = tokio::spawn(async move {
        debug!("Message receiver task started");

        while counter_clone.load(Ordering::SeqCst) < messages {
            match receive_tlv_message(&mut recv_reader).await {
                Some(msg) => {
                    trace!("Received message: event_type={:?}", msg.event_type);

                    if msg.event_type == EventType::Event {
                        if let Some(k) = msg.get_field(FieldType::Key) {
                            if let Ok(key_str) = std::str::from_utf8(k) {
                                if key_str == topic {
                                    // Get message number from the value
                                    if let Some(value) = msg.get_field(FieldType::Value) {
                                        if let Ok(value_str) = std::str::from_utf8(value) {
                                            if let Some(msg_num_str) = value_str.strip_prefix("msg ") {
                                                if let Ok(msg_num) = msg_num_str.parse::<usize>() {
                                                    trace!("Processing message {}", msg_num);

                                                    // Get the timestamp when this specific message was sent
                                                    let sent_ts = {
                                                        let timestamps = match message_timestamps_clone.lock() {
                                                            Ok(guard) => guard,
                                                            Err(e) => {
                                                                error!("Failed to lock message timestamps: {}", e);
                                                                continue;
                                                            }
                                                        };
                                                        timestamps.get(&msg_num).cloned().unwrap_or(0)
                                                    };

                                                    // Calculate latency immediately upon message receipt
                                                    let now_ts = match SystemTime::now().duration_since(UNIX_EPOCH) {
                                                        Ok(duration) => duration.as_millis() as u64,
                                                        Err(e) => {
                                                            error!("System time error: {}", e);
                                                            continue;
                                                        }
                                                    };

                                                    // Calculate latency
                                                    let latency_ms = if now_ts >= sent_ts && sent_ts > 0 {
                                                        now_ts - sent_ts
                                                    } else {
                                                        // This should rarely happen, but handle the case where clocks might be slightly off
                                                        warn!("Clock synchronization issue detected: now_ts={}, sent_ts={}", now_ts, sent_ts);
                                                        println!("Clock synchronization issue detected");
                                                        0
                                                    };

                                                    // Log every 50th message to reduce console output
                                                    if msg_num % 50 == 0 {
                                                        debug!("Message {}: latency={} ms", msg_num, latency_ms);
                                                        println!("Latency: {} ms (now: {}, sent: {}, msg: {})", 
                                                                 latency_ms, now_ts, sent_ts, msg_num);
                                                    }

                                                    latenzen.push(latency_ms);
                                                } else {
                                                    warn!("Failed to parse message number: {}", msg_num_str);
                                                }
                                            } else {
                                                warn!("Message value does not have expected format: {}", value_str);
                                            }
                                        } else {
                                            warn!("Message value is not valid UTF-8");
                                        }
                                    } else {
                                        warn!("Message has no value field");
                                    }

                                    counter_clone.fetch_add(1, Ordering::SeqCst);
                                    let count = counter_clone.load(Ordering::SeqCst);

                                    if count % 100 == 0 {
                                        info!("Received {}/{} messages", count, messages);
                                    }
                                } else {
                                    debug!("Ignoring message with different topic: {}", key_str);
                                }
                            } else {
                                warn!("Message key is not valid UTF-8");
                            }
                        } else {
                            warn!("Message has no key field");
                        }
                    } else {
                        debug!("Ignoring non-event message: {:?}", msg.event_type);
                    }
                },
                None => {
                    warn!("Connection closed by remote peer");
                    break;
                }
            }
        }

        info!("Message receiver task completed, processed {} messages", latenzen.len());
        latenzen
    });

    // Send messages one by one without batching
    info!("Starting to send {} messages individually", messages);
    println!("Sending {} messages individually...", messages);

    for i in 0..messages {
        trace!("Creating message {}/{}", i+1, messages);
        let mut msg = TLVMessage::new(EventType::Event);
        msg.insert_field(FieldType::Key, Bytes::from(topic));
        msg.insert_field(FieldType::Value, Bytes::from(format!("msg {}", i)));

        // Get current timestamp right before sending and store it for this specific message
        let ts = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(duration) => duration.as_millis() as u64,
            Err(e) => {
                error!("System time error: {}", e);
                continue;
            }
        };

        // Store the timestamp for this message
        {
            match message_timestamps.lock() {
                Ok(mut timestamps) => {
                    timestamps.insert(i, ts);
                },
                Err(e) => {
                    error!("Failed to lock message timestamps: {}", e);
                    continue;
                }
            }
        }

        // Also include timestamp in the message for backward compatibility
        msg.insert_field(FieldType::Timestamp, Bytes::from(ts.to_be_bytes().to_vec()));

        let encoded = msg.encode();
        trace!("Sending message {}/{} ({} bytes)", i+1, messages, encoded.len());

        match send_writer.write_all(&encoded).await {
            Ok(_) => {
                // Flush after each message to ensure it's sent immediately
                match send_writer.flush().await {
                    Ok(_) => {
                        trace!("Message {}/{} sent and flushed", i+1, messages);
                    },
                    Err(e) => {
                        error!("Failed to flush after sending message {}: {}", i, e);
                        eprintln!("Error sending message {}: {}", i, e);
                    }
                }
            },
            Err(e) => {
                error!("Failed to send message {}: {}", i, e);
                eprintln!("Error sending message {}: {}", i, e);
            }
        }

        // Print progress every 100 messages
        if i > 0 && i % 100 == 0 {
            info!("Progress: {}/{} messages sent", i, messages);
            println!("Progress: {}/{} messages sent", i, messages);
        }
    }

    info!("All {} messages sent, waiting for receiver to complete", messages);

    let latenzen = match handle.await {
        Ok(latencies) => {
            info!("Receiver task completed successfully");
            latencies
        },
        Err(e) => {
            error!("Receiver task failed: {}", e);
            eprintln!("Error in receiver task: {}", e);
            Vec::new()
        }
    };

    debug!("Calculating performance statistics");
    let duration = start.elapsed();
    let seconds = duration.as_secs_f64();
    let rate = messages as f64 / seconds;
    info!("Test duration: {:.3} seconds, message rate: {:.1} msg/sec", seconds, rate);

    // Filter out extreme outliers (values more than 3 times the median)
    // to get more accurate statistics
    debug!("Sorting latency values for statistical analysis");
    let mut sorted_latenzen = latenzen.clone();
    sorted_latenzen.sort();

    let median = if sorted_latenzen.is_empty() {
        debug!("No latency data available");
        0
    } else if sorted_latenzen.len() % 2 == 0 {
        let m1 = sorted_latenzen[sorted_latenzen.len() / 2 - 1];
        let m2 = sorted_latenzen[sorted_latenzen.len() / 2];
        debug!("Calculating median from even number of samples: ({} + {}) / 2", m1, m2);
        (m1 + m2) / 2
    } else {
        let m = sorted_latenzen[sorted_latenzen.len() / 2];
        debug!("Median from odd number of samples: {}", m);
        m
    };

    // Filter out extreme outliers for a more accurate average
    debug!("Filtering outliers (> {}ms) for average calculation", median * 3);
    let filtered_latenzen: Vec<u64> = latenzen.iter()
        .filter(|&&latenz| latenz <= median * 3)
        .cloned()
        .collect();

    let avg = if filtered_latenzen.is_empty() {
        debug!("No filtered latency data available");
        0.0
    } else {
        let sum: u64 = filtered_latenzen.iter().sum();
        debug!("Calculating average from {} filtered samples, sum: {}", filtered_latenzen.len(), sum);
        sum as f64 / filtered_latenzen.len() as f64
    };

    let min = *latenzen.iter().min().unwrap_or(&0);
    let max = *latenzen.iter().max().unwrap_or(&0);
    debug!("Min latency: {}ms, Max latency: {}ms", min, max);

    // Calculate 95th percentile for a better understanding of real-world performance
    let p95 = if sorted_latenzen.is_empty() {
        debug!("Cannot calculate 95th percentile, no data");
        0
    } else {
        let idx = (sorted_latenzen.len() as f64 * 0.95) as usize;
        let p95_value = sorted_latenzen[idx.min(sorted_latenzen.len() - 1)];
        debug!("95th percentile at index {}: {}ms", idx, p95_value);
        p95_value
    };

    // Log the summary statistics
    info!("Latency statistics: avg={:.1}ms, min={}ms, max={}ms, median={}ms, p95={}ms", 
          avg, min, max, median, p95);
    info!("Processed {} messages ({:.1}% of sent messages)", 
          latenzen.len(), (latenzen.len() as f64 / messages as f64 * 100.0));

    // Display results to the user
    println!("Duration: {:.3} seconds", seconds);
    println!("Messages/second: {:.1}", rate);
    println!("Latency: avg {:.1} ms, min {} ms, max {} ms, median {} ms, p95 {} ms", 
             avg, min, max, median, p95);
    println!("Processed: {} messages ({}% of sent)", 
             latenzen.len(), (latenzen.len() as f64 / messages as f64 * 100.0) as u64);

    // Report on outliers and filtering
    let outliers = latenzen.len() - filtered_latenzen.len();
    if outliers > 0 {
        info!("{} outliers filtered for average calculation (>{} ms)", outliers, median * 3);
        println!("{} outliers were filtered for average calculation (>{} ms)", 
                 outliers, median * 3);
    }

    info!("Performance test completed");
    println!("Performance test completed");

    // Explanation of the latency calculation
    println!("\nNotes on latency calculation:");
    println!("- Each message is sent individually and immediately flushed");
    println!("- Timestamps are captured immediately before sending each message and stored per message");
    println!("- Each message is measured individually, with exact mapping of send and receive timestamps");
    println!("- Latency is calculated separately for each message, based on its individual send time");
    println!("- Extreme outliers (>3x median) are filtered for average calculation");
    println!("- The 95th percentile shows typical latency under real conditions");
}

async fn send_ping(mut writer: BufWriter<tokio::net::tcp::OwnedWriteHalf>) {
    debug!("Creating ping message");
    let mut msg = TLVMessage::new(EventType::Event);
    msg.insert_field(FieldType::Key, Bytes::from("ping/test"));
    msg.insert_field(FieldType::Value, Bytes::from("ping!"));

    let encoded = msg.encode();
    debug!("Sending ping message ({} bytes)", encoded.len());

    match writer.write_all(&encoded).await {
        Ok(_) => {
            match writer.flush().await {
                Ok(_) => {
                    info!("Ping message sent successfully");
                    println!("Ping sent");
                },
                Err(e) => {
                    error!("Failed to flush after sending ping: {}", e);
                    eprintln!("Error sending ping: {}", e);
                }
            }
        },
        Err(e) => {
            error!("Failed to send ping message: {}", e);
            eprintln!("Error sending ping: {}", e);
        }
    }
}

async fn chat_mode(mut reader: BufReader<tokio::net::tcp::OwnedReadHalf>, mut writer: BufWriter<tokio::net::tcp::OwnedWriteHalf>) {
    info!("Starting chat mode receiver task");
    tokio::spawn(async move {
        debug!("Chat receiver task started");
        loop {
            match receive_tlv_message(&mut reader).await {
                Some(msg) => {
                    let key = msg.get_field(FieldType::Key)
                        .and_then(|k| std::str::from_utf8(k).ok())
                        .unwrap_or("<unknown>");

                    if let Some(value) = msg.get_field(FieldType::Value) {
                        if let Ok(text) = std::str::from_utf8(value) {
                            debug!("Received message on topic {}: {}", key, text);
                            println!("[{}]: {}", key, text);
                        } else {
                            warn!("Received message with non-UTF8 content on topic {}", key);
                        }
                    } else {
                        warn!("Received message without value field on topic {}", key);
                    }
                }
                None => {
                    info!("Connection closed by remote peer");
                    println!("Connection closed");
                    break;
                }
            }
        }
    });

    info!("Chat mode sender ready");
    loop {
        let input = prompt("Message: ");
        if input.trim().is_empty() {
            trace!("Empty message input, ignoring");
            continue;
        }

        debug!("Creating chat message");
        let mut msg = TLVMessage::new(EventType::Event);
        msg.insert_field(FieldType::Key, Bytes::from("chat/global"));
        msg.insert_field(FieldType::Value, Bytes::from(input.clone()));
        let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        msg.insert_field(FieldType::Timestamp, Bytes::from(ts.to_be_bytes().to_vec()));

        let encoded = msg.encode();
        debug!("Sending chat message ({} bytes): {}", encoded.len(), input);

        match writer.write_all(&encoded).await {
            Ok(_) => {
                match writer.flush().await {
                    Ok(_) => {
                        debug!("Chat message sent successfully");
                    },
                    Err(e) => {
                        error!("Failed to flush after sending chat message: {}", e);
                        eprintln!("Error sending message: {}", e);
                    }
                }
            },
            Err(e) => {
                error!("Failed to send chat message: {}", e);
                eprintln!("Error sending message: {}", e);
                break;
            }
        }
    }
}

async fn send_custom_event(mut writer: BufWriter<tokio::net::tcp::OwnedWriteHalf>) {
    let topic = prompt("Topic: ");
    info!("Preparing custom event for topic: {}", topic);

    let payload = prompt("Payload: ");
    debug!("Creating custom event message");

    let mut msg = TLVMessage::new(EventType::Event);
    msg.insert_field(FieldType::Key, Bytes::from(topic.clone()));
    msg.insert_field(FieldType::Value, Bytes::from(payload.clone()));

    let encoded = msg.encode();
    debug!("Sending custom event ({} bytes) to topic: {}", encoded.len(), topic);

    match writer.write_all(&encoded).await {
        Ok(_) => {
            match writer.flush().await {
                Ok(_) => {
                    info!("Custom event sent successfully to topic: {}", topic);
                    println!("Event sent to topic: {}", topic);
                },
                Err(e) => {
                    error!("Failed to flush after sending custom event: {}", e);
                    eprintln!("Error sending event: {}", e);
                }
            }
        },
        Err(e) => {
            error!("Failed to send custom event: {}", e);
            eprintln!("Error sending event: {}", e);
        }
    }
}

async fn subscribe_listen(mut reader: BufReader<tokio::net::tcp::OwnedReadHalf>, mut writer: BufWriter<tokio::net::tcp::OwnedWriteHalf>) {
    let channel = prompt("Channel: ");
    info!("Subscribing to channel: {}", channel);

    debug!("Creating subscription message");
    let mut msg = TLVMessage::new(EventType::Subscribe);
    msg.insert_field(FieldType::Key, Bytes::from(channel.clone()));

    let encoded = msg.encode();
    debug!("Sending subscription request ({} bytes)", encoded.len());

    match writer.write_all(&encoded).await {
        Ok(_) => {
            match writer.flush().await {
                Ok(_) => {
                    info!("Subscribed to channel: {}", channel);
                    println!("Subscribed to channel: {}", channel);
                    println!("Listening for messages...");
                },
                Err(e) => {
                    error!("Failed to flush after sending subscription: {}", e);
                    eprintln!("Error subscribing: {}", e);
                    return;
                }
            }
        },
        Err(e) => {
            error!("Failed to send subscription request: {}", e);
            eprintln!("Error subscribing: {}", e);
            return;
        }
    }

    info!("Starting message listener for channel: {}", channel);
    loop {
        match receive_tlv_message(&mut reader).await {
            Some(msg) => {
                let key = msg.get_field(FieldType::Key)
                    .and_then(|k| std::str::from_utf8(k).ok())
                    .unwrap_or("<unknown>");

                if let Some(value) = msg.get_field(FieldType::Value) {
                    if let Ok(text) = std::str::from_utf8(value) {
                        debug!("Received message on topic {}: {}", key, text);
                        println!("[{}]: {}", key, text);
                    } else {
                        warn!("Received message with non-UTF8 content on topic {}", key);
                    }
                } else {
                    warn!("Received message without value field on topic {}", key);
                }
            },
            None => {
                info!("Connection closed by remote peer");
                println!("Connection closed");
                break;
            }
        }
    }
}

async fn receive_tlv_message<R: AsyncReadExt + Unpin>(reader: &mut R) -> Option<TLVMessage> {
    trace!("Reading message header (4 bytes)");
    let mut buf = [0u8; 4];
    if let Err(e) = reader.read_exact(&mut buf).await {
        debug!("Failed to read message header: {}", e);
        return None;
    }

    let len = u32::from_be_bytes(buf) as usize;
    if len < 5 || len > MAX_MESSAGE_SIZE {
        warn!("Invalid message length: {} (min: 5, max: {})", len, MAX_MESSAGE_SIZE);
        return None;
    }

    let payload_len = len - 4;
    trace!("Reading message body ({} bytes)", payload_len);

    let mut body = vec![0u8; payload_len];
    if let Err(e) = reader.read_exact(&mut body).await {
        debug!("Failed to read message body: {}", e);
        return None;
    }

    let mut full = Vec::with_capacity(len);
    full.extend_from_slice(&buf);
    full.extend_from_slice(&body);

    match TLVMessage::parse(Bytes::from(full)) {
        Ok(msg) => {
            trace!("Successfully parsed message: event_type={:?}", msg.event_type);
            Some(msg)
        },
        Err(e) => {
            warn!("Failed to parse message: {}", e);
            None
        }
    }
}

fn prompt(label: &str) -> String {
    print!("{label}");
    if let Err(e) = stdout().flush() {
        warn!("Failed to flush stdout: {}", e);
    }

    let mut input = String::new();
    match stdin().read_line(&mut input) {
        Ok(_) => {},
        Err(e) => {
            error!("Failed to read input: {}", e);
            return String::new();
        }
    }

    input.trim().to_string()
}
