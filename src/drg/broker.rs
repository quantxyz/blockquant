// Copyright (c) 2024 quantxyz@drg.com
// All rights reserved.

// Author: quantxyz
// Email: lktsepc@gmail.com

use super::model::{Candle, CandleHelper, Event};
use crate::utils::db::ClientMongo;
use mongodb::bson::{self, doc};
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct BrokerLocal {
    pub event_sender: mpsc::UnboundedSender<Event>,
}

pub async fn get_candles(
    symbol: &str,
    interval: &str,
    timestamp_start: i64,
    timestamp_end: i64,
) -> Vec<Candle> {
    let label = format!("{}_{}", symbol, interval);
    let mut filter = doc! {"_id": {"$gt": timestamp_start}};
    if timestamp_end > 0 && timestamp_end > timestamp_start {
        let end_condition = doc! {"_id": {"$lt": timestamp_end}};
        let and_filter = doc! {"$and": vec![filter, end_condition]};
        filter = and_filter;
    }
    let client = ClientMongo::with_db_name("cryptodb".to_string());
    let result = client.records_query(&label, Some(filter), None, None, None).await;
    match result {
        Ok(records) => {
            let candles: Vec<Candle> = records
                .into_iter()
                .filter_map(|doc| {
                    // 尝试从Document中反序列化CandleHelper
                    let candle_helper: CandleHelper = match bson::from_document(doc) {
                        Ok(helper) => helper,
                        Err(e) => {
                            log::error!("{:?}", e);
                            return None;
                        } // 如果反序列化失败，跳过这个文档
                    };

                    // 手动转换CandleHelper为Candle
                    Some(Candle {
                        symbol: symbol.to_string(),
                        timestamp: candle_helper.timestamp,
                        open: candle_helper.open,
                        high: candle_helper.high,
                        low: candle_helper.low,
                        close: candle_helper.close,
                        volume: candle_helper.volume,
                        interval: candle_helper.interval,
                    })
                })
                .collect();
            candles
        }
        Err(e) => {
            log::error!("get_candles error: {:?}", e);
            Vec::new()
        }
    }
}

impl BrokerLocal {
    pub fn new(event_sender: mpsc::UnboundedSender<Event>) -> Self {
        BrokerLocal {
            event_sender,
        }
    }

    pub async fn start(
        &self,
        symbols: &Vec<String>,
        intervals: &Vec<String>,
        items_timestamp_start: &std::collections::HashMap<String, i64>,
        items_timestamp_end: &std::collections::HashMap<String, i64>,
    ) {
        let mut tasks = vec![];
        for s in symbols {
            let symbol = s.clone();
            let intervals = intervals.clone();
            let items_timestamp_start = items_timestamp_start.clone();
            let items_timestamp_end = items_timestamp_end.clone();
            let event_sender = self.event_sender.clone();

            let task = tokio::spawn(async move {
                for interval in intervals {
                    if [
                        "1m", "3m", "5m", "15m", "30m", "1h", "2h", "4h", "6h", "8h", "12h", "1d",
                        "3d", "1w", "1M",
                    ]
                    .contains(&interval.as_str())
                    {
                        let item = format!("{}_{}", symbol, interval);
                        let timestamp_start = items_timestamp_start.get(&item).unwrap_or(&0).to_owned();
                        let timestamp_end = items_timestamp_end.get(&item).unwrap_or(&0).to_owned();
                        let _candles = get_candles(&symbol, &interval, timestamp_start, timestamp_end).await;
                        let candles_ref = if _candles.len() >= 1 {
                            &_candles[.._candles.len() - 1]
                        } else {
                            &[]
                        };
                        let candles: Vec<Candle> = candles_ref.to_vec();
                        for candle in candles {
                            let _t = if candle.timestamp.to_string().len() == 10 {
                                candle.timestamp * 1000
                            } else {
                                candle.timestamp
                            };
                            let c = Candle {
                                symbol: candle.symbol,
                                timestamp: _t,
                                open: candle.open,
                                high: candle.high,
                                low: candle.low,
                                close: candle.close,
                                volume: candle.volume,
                                interval: candle.interval,
                            };
                            // Call on_candle_event
                            if let Err(e) = event_sender.send(Event::EventCandle(c)) {
                                eprintln!("Failed to send candle event: {}", e);
                            }
                        }
                    }
                }
            });
            tasks.push(task);
        }

        for task in tasks {
            let _ = task.await;
        }
        
    }
}
