use chrono::{TimeZone, Utc, DateTime};
use std::collections::HashMap;
mod drg;
mod utils;
use drg::{
    model::{Candle, Equity, Order, Position, TradeRecord, StrategyParams},
    strategy::{IStgHandler, Strategy},
};
use utils::{logger, common};
// use drg::model::{Candle, Equity, Order, Position, TradeRecord};
// use drg::strategy::{IStgHandler, Strategy};
use async_trait::async_trait;


#[async_trait]
impl IStgHandler for Strategy {
    async fn on_init(&mut self) {
        log::info!("on_init");
    }
    async fn on_finish(&mut self) {
        log::info!("on_finish");
    }
    async fn on_candle(&mut self, candle: &Candle) {
        let item = format!("{}_{}", candle.symbol, candle.interval);
        let timestamp_millis = candle.timestamp;
        let datetime: DateTime<Utc> = match Utc.timestamp_millis_opt(timestamp_millis) {
            chrono::LocalResult::Single(datetime) => datetime,
            chrono::LocalResult::Ambiguous(_, _) => {
                println!("Ambiguous timestamp");
                return;
            },
            chrono::LocalResult::None => {
                println!("Invalid timestamp");
                return;
            },
        };
        // println!("get {}", item);
        let len = self
            .context
            .candles
            .get(&item)
            .map(|candles| candles.len())
            .unwrap_or(0);
        // log::info!("{}, Stg, {} No.{}", datetime.format("%Y-%m-%d %H:%M:%S%.3f UTC"), item, len);

    }
    async fn on_trade_record(&mut self, _: &TradeRecord) {}
    async fn on_equity(&mut self, _: &Equity) {}
    async fn on_order(&mut self, order: &Order) {
        let now = Utc::now();
        // 将当前时间转换为时间戳（以毫秒为单位）
        let current_timestamp_millis = now.timestamp_millis();
            // 计算两个时间戳之间的差值（以毫秒为单位）
        let diff_millis = (current_timestamp_millis - order.timestamp).abs();

        // 将3天转换为毫秒
        let three_days_millis = 3 * 24 * 60 * 60 * 1000;

        let datetime = common::timestamp_millis_to_datetime(order.timestamp);

        let action = if order.qty > 0.00 {
            "Long"
        } else {
            "Short"
        };
        
        log::info!("{},{}@{},{}", datetime.format("%Y-%m-%d %H:%M:%S%.3f UTC"), action, order.item, order.price);
        // filter 判断差值是否大于3天
        // if diff_millis < three_days_millis {
        //     log::info!("{},{}@{},{}", datetime.format("%Y-%m-%d %H:%M:%S%.3f UTC"), action, order.item, order.price);
        // }
    }
    async fn on_position(&mut self, _: &Position) {}
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    logger::setup("log", "stg.log", false).expect("config log sys failed");

    // let _is_ml = false;
    let timestamp_start = Utc
        .with_ymd_and_hms(2024, 1, 1, 0, 0, 0)
        .unwrap()
        .timestamp()
        * 1000;
    let timestamp_end = Utc
        .with_ymd_and_hms(2025, 1, 16, 0, 0, 0)
        .unwrap()
        .timestamp()
        * 1000;

    let mut items_timestamp_start: HashMap<String, i64> = HashMap::new();
    let mut items_timestamp_end: HashMap<String, i64> = HashMap::new();

    let symbols = vec!["BTCUSDT".to_string(), "ETHUSDT".to_string()];
    let intervals = vec!["1d".to_string()];
    for symbol in &symbols {
        for interval in &intervals {
            let key = format!("{}_{}", symbol, interval);
            items_timestamp_start.insert(key.clone(), timestamp_start - 1000);
            items_timestamp_end.insert(key, timestamp_end + 1000);
        }
    }
    let window = 10;
    let initial_capital = 4000.00;
    let _is_ml = false;
    let params = StrategyParams {
        stg_name: format!("SuperTrend{}{}", window, if _is_ml { "ML" } else { "" }),
        window_length: window * 2,
        window_atr: window,
        is_use_percent_of_equity: false,
        percent_of_equity: 0.5,
        percent_of_every_trade_money: 0.03,
        is_sl: true,
        n_atr_sl: 2.0,
        is_tp: false,
        n_atr_tp: 5.0,
        tp_method: "percent_0.23".to_string(),
        symbols,
        intervals,
        initial_capital,
        items_timestamp_start,
        items_timestamp_end,
        trading_fee: 0.001,
    };
    
    let mut stg = Strategy::new(params);
    stg.run().await;
    Ok(())
}
