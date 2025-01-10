use chrono::{TimeZone, Utc};
use async_trait::async_trait;
use std::collections::HashMap;
mod drg;
mod utils;
use drg::model::{Candle, Equity, Order, Position, TradeRecord, StrategyParams};
use drg::strategy::{IStgHandler, Strategy};
use utils::{logger, common};
use polars::prelude::{DataFrame, Series, NamedFrom};


fn true_range(current: &Candle, previous: &Candle) -> f64 {
    let range1 = current.high - current.low;
    let range2 = (current.high - previous.close).abs();
    let range3 = (current.low - previous.close).abs();
    range1.max(range2).max(range3)
}

fn calculate_atr(data: &[Candle], period: usize) -> Vec<f64> {
    if data.len() < period + 1 {
        return Vec::<f64>::new();
    }

    // 计算 TR 值
    let mut tr_values = Vec::new();
    for i in 1..data.len() {
        let current = &data[i];
        let previous = &data[i - 1];
        let tr = true_range(current, previous);
        tr_values.push(tr);
    }

    // 计算 ATR 值
    let mut atr_values = Vec::new();
    let mut sum_tr = 0.0;
    for i in 0..tr_values.len() {
        sum_tr += tr_values[i];
        if i >= period - 1 {
            if i >= period {
                sum_tr -= tr_values[i - period];
            }
            atr_values.push(sum_tr / period as f64);
        }
    }

    atr_values
}

fn candles_to_dataframe(mut _candles: Vec<Candle>) -> DataFrame {
    let len = _candles.len();
    let candles;
    if len <= 1000 {
        candles = _candles.clone() // 或者 vec.to_vec()，如果需要所有权转移
    } else {
        let new_len = len - 1000;
        candles = _candles.split_off(new_len)
    }

    let symbols: Vec<String> = candles.iter().map(|c| c.symbol.clone()).collect();
    let timestamps: Vec<i64> = candles.iter().map(|c| c.timestamp).collect();
    let opens: Vec<f64> = candles.iter().map(|c| c.open).collect();
    let highs: Vec<f64> = candles.iter().map(|c| c.high).collect();
    let lows: Vec<f64> = candles.iter().map(|c| c.low).collect();
    let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
    let volumes: Vec<f64> = candles.iter().map(|c| c.volume).collect();
    let intervals: Vec<String> = candles.iter().map(|c| c.interval.clone()).collect();

    let df = DataFrame::new(vec![
        Series::new("symbol", symbols),
        Series::new("timestamp", timestamps),
        Series::new("open", opens),
        Series::new("high", highs),
        Series::new("low", lows),
        Series::new("close", closes),
        Series::new("volume", volumes),
        Series::new("interval", intervals),
    ]);

    return df.expect("candles to df failed");
}


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
        let close = candle.close;
        let high = candle.high;
        let low = candle.low;

        let timestamp_millis = candle.timestamp;
        let len = self
            .context
            .candles
            .get(&item)
            .map(|candles| candles.len())
            .unwrap_or(0);
        if len < 21 {
            return;
        }
        
        let _candles = self.context.candles.get(&item);
        if let Some(candles_ref) = _candles {
            // let df = candles_to_dataframe(candles_ref.to_vec()); 
            let candles = candles_ref.to_vec();
            let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
            let period = 20;
            let atrs = calculate_atr(&candles, period);
            if let Some(atr) = atrs.last() {
                self.context.update_atr(&item, *atr);
            }
            if let Some(value) = self.context.get_atr(&item) {
                // println!("{}, ATR_{}:{:?}", item, period, value);
            }
            let sma = common::calculate_sma(&closes, period);
            if let Some(value) = sma.last() {
                // println!("{}, SMA_{}:{:?}", item, period, value);
            }
            
            let ema = common::calculate_ema(&closes, period);
            if let Some(value) = ema.last() {
                // println!("{}, EMA_{}:{:?}", item, period, value);
            }
            let mut pos_size = 0.0;
            if let Some(last_pos) = self.context.get_position(&item) {
                pos_size = last_pos.size;
            }
            let highs: Vec<f64> = candles.iter().map(|c| c.high).collect();
            let lows: Vec<f64> = candles.iter().map(|c| c.low).collect();
            let max = common::find_max_last_n(&highs, 20);
            if max > 0.0 {
                // println!("{}, MAX_{}:{:?}", item, period, value);
                if high == max && pos_size <= 0.0 {
                    self.buy(&item, close, timestamp_millis, Some(100.00)).await;
                }
            }
            
            let min = common::find_min_last_n(&lows, 20);
            if min > 0.0 {
                // println!("{}, MIN_{}:{:?}", item, period, value);
                if low == min && pos_size >= 0.0 {
                    self.sell(&item, close, timestamp_millis, Some(100.00)).await;
                }
            }
            
        }
    }
    async fn on_trade_record(&mut self, trade_record: &TradeRecord) {}
    async fn on_equity(&mut self, equity: &Equity) {
        let datetime = common::timestamp_millis_to_datetime(equity.timestamp);
        // log::info!("{},{},eq:{},aval:{}", 
        //     datetime.format("%Y-%m-%d %H:%M:%S%.3f UTC"), 
        //     equity.item, 
        //     equity.equity_value,
        //     equity.cash_aval
        // );
    }
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
        // // 判断差值是否大于3天
        // if diff_millis < three_days_millis {
        //     log::info!("{},{}@{},{}", datetime.format("%Y-%m-%d %H:%M:%S%.3f UTC"), action, order.item, order.price);
        // }
    }
    async fn on_position(&mut self, position: &Position) {
        // let datetime = common::timestamp_millis_to_datetime(position.timestamp);
        // log::info!("{}, {}, pos_size: {}, avg_price:{}, stop_loss:{}, take_profit:{}", 
        //     datetime.format("%Y-%m-%d %H:%M:%S%.3f UTC"), 
        //     position.item, 
        //     position.size, 
        //     position.price, 
        //     position.stop_loss, 
        //     position.take_profit);
    }
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    logger::setup("log", "stg.log", false).expect("config log sys failed");

    let timestamp_start = Utc
        .with_ymd_and_hms(2024, 1, 1, 0, 0, 0)
        .unwrap()
        .timestamp()
        * 1000;
    let timestamp_end = Utc
        .with_ymd_and_hms(2025, 1, 10, 0, 0, 0)
        .unwrap()
        .timestamp()
        * 1000;

    let mut items_timestamp_start: HashMap<String, i64> = HashMap::new();
    let mut items_timestamp_end: HashMap<String, i64> = HashMap::new();

    let symbols = vec!["BTCUSDT", "ETHUSDT"].into_iter().map(String::from).collect();
    let intervals = vec!["1d".to_string()];
    for symbol in &symbols {
        for interval in &intervals {
            let key = format!("{}_{}", symbol, interval);
            items_timestamp_start.insert(key.clone(), timestamp_start - 1000);
            items_timestamp_end.insert(key, timestamp_end + 1000);
        }
    }
    let window = 20;
    let initial_capital = 4000.00;
    let _is_ml = false;
    let params = StrategyParams {
        stg_name: format!("SuperTrend{}{}", window, if _is_ml { "ML" } else { "" }),
        window_length: window,
        window_atr: window,
        is_use_percent_of_equity: false,
        percent_of_equity: 0.5,
        percent_of_every_trade_money: 0.03,
        is_sl: true,
        n_atr_sl: 2.00,
        is_tp: false,
        n_atr_tp: 5.00,
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
