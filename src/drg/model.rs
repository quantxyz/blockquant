use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

// 自定义反序列化函数，用于将字符串转换为f64
fn deserialize_f64<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = String::deserialize(deserializer)?;
    s.parse::<f64>().map_err(serde::de::Error::custom)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Candle {
    pub symbol: String,
    pub timestamp: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub interval: String,
}

#[derive(Debug, Deserialize)]
pub struct CandleHelper {
    pub timestamp: i64,
    #[serde(deserialize_with = "deserialize_f64")]
    pub open: f64,
    #[serde(deserialize_with = "deserialize_f64")]
    pub high: f64,
    #[serde(deserialize_with = "deserialize_f64")]
    pub low: f64,
    #[serde(deserialize_with = "deserialize_f64")]
    pub close: f64,
    #[serde(deserialize_with = "deserialize_f64")]
    pub volume: f64,
    pub interval: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Position {
    pub item: String,
    pub size: f64,
    pub price: f64,
    pub highest: f64,
    pub lowest: f64,
    pub stop_loss: f64,
    pub take_profit: f64,
    pub timestamp: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Order {
    pub item: String,
    pub price: f64,
    pub qty: f64,
    pub timestamp: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Equity {
    pub item: String,
    pub timestamp: i64,
    pub equity_value: f64,
    pub close_latest: f64,
    pub pos_size: f64,
    pub cash_aval: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TradeRecord {
    pub item: String,
    pub side: String,
    pub size: f64,
    pub price_open: f64,
    pub time_open: i64,
    pub price_close: f64,
    pub time_close: i64,
    pub label_close: String,
}

pub enum Event {
    EventFinish(),
    EventCandle(Candle),
    EventPosition(Position),
    EventOrder(Order),
    EventEquity(Equity),
    EventTradeRecord(TradeRecord),
}

#[derive(Debug)]
pub struct Context {
    pub candles: HashMap<String, Vec<Candle>>,
    pub positions: HashMap<String, Position>,
    pub trade_records: HashMap<String, Vec<TradeRecord>>,
    pub equities: HashMap<String, Vec<Equity>>,
    pub atrs: HashMap<String, f64>,
    pub profits: Vec<Profit>,
}

impl Context {
    pub fn new() -> Self {
        Context {
            candles: HashMap::new(),
            positions: HashMap::new(),
            trade_records: HashMap::new(),
            equities: HashMap::new(),
            atrs: HashMap::new(),
            profits: Vec::new(),
        }
    }
    pub fn push_candle(&mut self, candle: Candle) {
        let item = format!("{}_{}", candle.symbol, candle.interval);
        self.candles
            .entry(item)
            .or_insert_with(Vec::new)
            .push(candle);
    }
    pub fn push_equity(&mut self, equity: Equity) {
        self.equities
            .entry(equity.item.to_string())
            .or_insert_with(Vec::new)
            .push(equity);
    }
    pub fn push_trade_record(&mut self, trade_record: TradeRecord) {
        self.trade_records
            .entry(trade_record.item.to_string())
            .or_insert_with(Vec::new)
            .push(trade_record);
    }

    pub fn update_trade_record(&mut self, trade_record: TradeRecord) {
        self.trade_records
            .entry(trade_record.item.to_string())
            .and_modify(|v| {
                if let Some(last) = v.last_mut() {
                    *last = trade_record;
                }
            });
    }
    pub fn get_last_equity(&mut self, item: &str) -> Option<&Equity> {
        self.equities.get(item).and_then(|v| v.last())
    }
    pub fn get_last_trade_record(&mut self, item: &str) -> Option<&TradeRecord> {
        self.trade_records.get(item).and_then(|v| v.last())
    }
    pub fn update_position(&mut self, position: Position) {
        let p = position.clone();
        self.positions.entry(position.item.to_string()).and_modify(|e| *e = position).or_insert(p);
    }
    pub fn update_atr(&mut self, item: &str, atr: f64) {
        self.atrs.entry(item.to_string()).and_modify(|e| *e = atr).or_insert(atr);
    }
    pub fn get_position(&self, item: &str) -> Option<&Position> {
        self.positions.get(item)
    }
    pub fn get_atr(&self, item: &str) -> Option<f64> {
        self.atrs.get(item).cloned()
    }
    pub fn push_profit(&mut self, profit: Profit) {
        self.profits.push(profit);
    }
}
#[derive(Debug)]
pub struct Profit {
    item: String,
    is_use_percent_of_equity: bool,
    percent_of_every_trade_money: f64,
    percent_of_equity: f64,
    initial_capital: f64,
    every_trade_fee: f64,
}

#[derive(Debug, Clone)]
pub struct StrategyParams {
    pub stg_name: String,
    pub window_length: i32,
    pub window_atr: i32,
    pub symbols: Vec<String>,
    pub intervals: Vec<String>,
    pub is_use_percent_of_equity: bool,
    pub percent_of_equity: f64,
    pub percent_of_every_trade_money: f64,
    pub is_sl: bool,
    pub n_atr_sl: f64,
    pub is_tp: bool,
    pub n_atr_tp: f64,
    pub tp_method: String,
    pub initial_capital: f64,
    pub items_timestamp_start: HashMap<String, i64>,
    pub items_timestamp_end: HashMap<String, i64>,
    pub trading_fee: f64,
}
