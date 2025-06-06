
use tokio::net::TcpStream;
use tokio::io::{AsyncWriteExt, AsyncReadExt, BufReader, BufWriter};
use bytes::Bytes;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use std::io::{stdin, stdout, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use Eventrelay::tlv::message::TLVMessage;
use Eventrelay::tlv::types::{EventType, FieldType};

const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024;

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1";
    let port = prompt("💬 Port eingeben: ");
    let full_addr = format!("{}:{}", addr, port);

    println!("🔧 Modus wählen:");
    println!("1 = Ping");
    println!("2 = Subscribe + Chat");
    println!("3 = Event senden");
    println!("4 = Performance-Test");
    println!("5 = Subscribe + Listen");
    use std::time::Instant;

    let mode = prompt("👉 Auswahl (1–5): ");

    match mode.trim() {
        "4" => dual_client_performance_test(full_addr).await,
        _ => {
            match TcpStream::connect(&full_addr).await {
                Ok(stream) => {
                    stream.set_nodelay(true).unwrap();
                    let (reader_raw, writer_raw) = stream.into_split();
                    let reader = BufReader::new(reader_raw);
                    let writer = BufWriter::new(writer_raw);

                    match mode.trim() {
                        "1" => send_ping(writer).await,
                        "2" => chat_mode(reader, writer).await,
                        "3" => send_custom_event(writer).await,
                        "5" => subscribe_listen(reader, writer).await,
                        _ => println!("❌ Ungültige Auswahl"),
                    }
                }
                Err(e) => eprintln!("❌ Verbindung fehlgeschlagen: {e}"),
            }
        }
    }
}

async fn dual_client_performance_test(addr: String) {
    let start = Instant::now();
    let topic = "perf/test";
    let messages = 10;
    let counter = Arc::new(AtomicUsize::new(0));

    let stream_recv = TcpStream::connect(&addr).await.unwrap();
    let stream_send = TcpStream::connect(&addr).await.unwrap();

    let (recv_reader_raw, recv_writer_raw) = stream_recv.into_split();
    let mut recv_reader = BufReader::new(recv_reader_raw);
    let mut recv_writer = BufWriter::new(recv_writer_raw);

    let (_, send_writer_raw) = stream_send.into_split();
    let mut send_writer = BufWriter::new(send_writer_raw);

    // Subscribe on both connections to ensure both client IDs are registered for the topic
    let mut sub = TLVMessage::new(EventType::Subscribe);
    sub.insert_field(FieldType::Key, Bytes::from(topic));

    // Subscribe on the receive connection
    recv_writer.write_all(&sub.encode()).await.unwrap();
    recv_writer.flush().await.unwrap();

    // Subscribe on the send connection
    send_writer.write_all(&sub.encode()).await.unwrap();
    send_writer.flush().await.unwrap();

    let counter_clone = counter.clone();
    let mut latenzen = Vec::with_capacity(messages);

    let handle = tokio::spawn(async move {
        while counter_clone.load(Ordering::SeqCst) < messages {
            if let Some(msg) = receive_tlv_message(&mut recv_reader).await {
                if msg.event_type == EventType::Event {
                    if let Some(k) = msg.get_field(FieldType::Key) {
                        if std::str::from_utf8(k).unwrap_or("") == topic {
                            if let Some(ts_bytes) = msg.get_field(FieldType::Timestamp) {
                                if ts_bytes.len() == 8 {
                                    let mut ts_array = [0u8; 8];
                                    ts_array.copy_from_slice(&ts_bytes[..]);
                                    let sent_ts = u64::from_be_bytes(ts_array);
                                    let now_ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
                                    let latency_ms = now_ts.saturating_sub(sent_ts);
                                    println!("⏱️ Latency: {} ms (now: {}, sent: {})", latency_ms, now_ts, sent_ts);
                                    latenzen.push(latency_ms);
                                }
                            }
                            counter_clone.fetch_add(1, Ordering::SeqCst);
                        }
                    }
                }
            } else {
                break;
            }
        }

        latenzen
    });

    for i in 0..messages {
        let mut msg = TLVMessage::new(EventType::Event);
        msg.insert_field(FieldType::Key, Bytes::from(topic));
        msg.insert_field(FieldType::Value, Bytes::from(format!("msg {}", i)));
        let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
        msg.insert_field(FieldType::Timestamp, Bytes::from(ts.to_be_bytes().to_vec()));
        send_writer.write_all(&msg.encode()).await.unwrap();
    }

    send_writer.flush().await.unwrap();
    let latenzen = handle.await.unwrap();

    let duration = start.elapsed();
    let seconds = duration.as_secs_f64();
    let rate = messages as f64 / seconds;

    let avg = latenzen.iter().sum::<u64>() as f64 / latenzen.len() as f64;
    let min = *latenzen.iter().min().unwrap_or(&0);
    let max = *latenzen.iter().max().unwrap_or(&0);

    println!("⏱️ Dauer: {:.3} Sekunden", seconds);
    println!("🚀 Nachrichten/Sekunde: {:.1}", rate);
    println!("📊 Latenz: Ø {:.1} ms, min {} ms, max {} ms", avg, min, max);
    println!("✅ Performance-Test abgeschlossen");
}

async fn send_ping(mut writer: BufWriter<tokio::net::tcp::OwnedWriteHalf>) {
    let mut msg = TLVMessage::new(EventType::Event);
    msg.insert_field(FieldType::Key, Bytes::from("ping/test"));
    msg.insert_field(FieldType::Value, Bytes::from("ping!"));

    let encoded = msg.encode();
    writer.write_all(&encoded).await.unwrap();
    writer.flush().await.unwrap();
    println!("📨 Ping gesendet");
}

async fn chat_mode(mut reader: BufReader<tokio::net::tcp::OwnedReadHalf>, mut writer: BufWriter<tokio::net::tcp::OwnedWriteHalf>) {
    tokio::spawn(async move {
        loop {
            match receive_tlv_message(&mut reader).await {
                Some(msg) => {
                    let key = msg.get_field(FieldType::Key).and_then(|k| std::str::from_utf8(k).ok()).unwrap_or("<unbekannt>");
                    if let Some(value) = msg.get_field(FieldType::Value) {
                        if let Ok(text) = std::str::from_utf8(value) {
                            println!("📥 [{key}]: {text}");
                        }
                    }
                }
                None => {
                    println!("🔌 Verbindung geschlossen");
                    break;
                }
            }
        }
    });

    loop {
        let input = prompt("✉ Nachricht: ");
        if input.trim().is_empty() {
            continue;
        }

        let mut msg = TLVMessage::new(EventType::Event);
        msg.insert_field(FieldType::Key, Bytes::from("chat/global"));
        msg.insert_field(FieldType::Value, Bytes::from(input));
        let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        msg.insert_field(FieldType::Timestamp, Bytes::from(ts.to_be_bytes().to_vec()));

        let encoded = msg.encode();
        writer.write_all(&encoded).await.unwrap();
        writer.flush().await.unwrap();
    }
}

async fn send_custom_event(mut writer: BufWriter<tokio::net::tcp::OwnedWriteHalf>) {
    let topic = prompt("🧩 Topic: ");
    let payload = prompt("📦 Payload: ");
    let mut msg = TLVMessage::new(EventType::Event);
    msg.insert_field(FieldType::Key, Bytes::from(topic));
    msg.insert_field(FieldType::Value, Bytes::from(payload));
    let encoded = msg.encode();
    writer.write_all(&encoded).await.unwrap();
    writer.flush().await.unwrap();
}

async fn subscribe_listen(mut reader: BufReader<tokio::net::tcp::OwnedReadHalf>, mut writer: BufWriter<tokio::net::tcp::OwnedWriteHalf>) {
    let channel = prompt("🔌 Channel: ");
    let mut msg = TLVMessage::new(EventType::Subscribe);
    msg.insert_field(FieldType::Key, Bytes::from(channel));
    writer.write_all(&msg.encode()).await.unwrap();
    writer.flush().await.unwrap();

    loop {
        if let Some(msg) = receive_tlv_message(&mut reader).await {
            let key = msg.get_field(FieldType::Key).and_then(|k| std::str::from_utf8(k).ok()).unwrap_or("<unbekannt>");
            if let Some(value) = msg.get_field(FieldType::Value) {
                if let Ok(text) = std::str::from_utf8(value) {
                    println!("📥 [{key}]: {text}");
                }
            }
        } else {
            println!("🔌 Verbindung geschlossen");
            break;
        }
    }
}

async fn receive_tlv_message<R: AsyncReadExt + Unpin>(reader: &mut R) -> Option<TLVMessage> {
    let mut buf = [0u8; 4];
    if reader.read_exact(&mut buf).await.is_err() {
        return None;
    }
    let len = u32::from_be_bytes(buf) as usize;
    if len < 5 || len > MAX_MESSAGE_SIZE {
        return None;
    }
    let payload_len = len - 4;
    let mut body = vec![0u8; payload_len];
    reader.read_exact(&mut body).await.ok()?;
    let mut full = Vec::with_capacity(len);
    full.extend_from_slice(&buf);
    full.extend_from_slice(&body);
    TLVMessage::parse(Bytes::from(full)).ok()
}

fn prompt(label: &str) -> String {
    print!("{label}");
    let _ = stdout().flush();
    let mut input = String::new();
    stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}
