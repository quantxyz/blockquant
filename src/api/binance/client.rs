// Copyright (c) 2024 quantxyz@drg.com
// All rights reserved.

// Author: quantxyz
// Email: lktsepc@gmail.com

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

// Broker结构体
struct Client {
    symbols: Vec<String>,
    intervals: Vec<String>,
    ws_url: String,
    event_sender: mpsc::UnboundedSender<Event>,
}

impl Client {
    // init
    async fn new(
        symbols: &[&str],
        intervals: &[&str],
        ws_url: String,
        event_sender: mpsc::UnboundedSender<Event>,
    ) -> Self {
        Self {
            symbols: symbols.iter().map(|&s| s.to_string()).collect(),
            intervals: intervals.iter().map(|&s| s.to_string()).collect(),
            ws_url,
            event_sender,
        }
    }

    // 异步接收事件,并传给Strategy
    async fn start(self) {
        loop {
            let ws_stream = match connect_async(&self.ws_url).await {
                Ok((ws_stream, _)) => ws_stream,
                Err(e) => {
                    log::err!("WebSocket conn err: {:?}", e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    continue;
                }
            };

            log::info!("WebSocket conn success");

            let (mut write, mut read) = ws_stream.split();

            // 发送订阅请求
            write.send(Message::Text("subscribe".to_string())).await;

            // 接收事件循环
            loop {
                let msg = tokio::select! {
                    Some(msg) = read.next() => msg,
                    else => break,
                };

                let msg = match msg {
                    Ok(msg) => msg,
                    Err(e) => {
                        log::err!("WebSocket msg err: {:?}", e);
                        break;
                    }
                };

                if let Message::Text(text) = msg {
                    // 解析收到的事件数据
                    let event = parse_event(&text);
                    // 发送事件到Strategy
                    self.event_sender.send(event).await;
                }
            }

            log::warn!("WebSocket conn closed, 5 seconds to reconnect...");
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        } // loop
    }
}
