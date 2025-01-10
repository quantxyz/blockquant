use super::broker::BrokerLocal;
use super::model::{Candle, Equity, Order, Position, TradeRecord, StrategyParams};
use super::model::{Context, Event};
use tokio::sync::mpsc::{self, error::TryRecvError};
use async_trait::async_trait;
use std::time::{Duration, Instant, SystemTime};
use tokio::task;

#[async_trait]
pub trait IStgHandler  {
    async fn on_init(&mut self);
    async fn on_candle(&mut self, candle: &Candle);
    async fn on_trade_record(&mut self, trade_record: &TradeRecord);
    async fn on_equity(&mut self, equity: &Equity);
    async fn on_order(&mut self, order: &Order);
    async fn on_position(&mut self, postion: &Position);
    async fn on_finish(&mut self);
}

fn get_timestamp_ms() -> i64 {
    let now = SystemTime::now();
    let since_the_epoch = now.duration_since(SystemTime::UNIX_EPOCH)
        .expect("Time went backwards");
    return since_the_epoch.as_millis() as i64;
}

pub struct Strategy {
    pub params: StrategyParams,
    pub context: Context,
    pub broker: BrokerLocal,
    pub event_receiver: mpsc::UnboundedReceiver<Event>,
}

impl Strategy {
    pub fn new(params: StrategyParams) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        let broker = BrokerLocal::new(sender);
        let context = Context::new();

        Strategy {
            params,
            context,
            broker,
            event_receiver: receiver,
        }
    }

    // 提取事件处理逻辑到一个单独的异步函数
    async fn handle_events(&mut self) {
        let mut start_time = Instant::now();
        loop {
            match self.event_receiver.try_recv() {
                Ok(event) => {
                    start_time = Instant::now();
                    match event {
                        Event::EventFinish() => {
                            self.on_finish().await;
                            break;
                        }
                        Event::EventCandle(candle) => {
                            self.context.push_candle(candle.clone());
                            self.on_candle(&candle).await;
                        },
                        Event::EventPosition(position) => {
                            self.on_position(&position).await;
                        },
                        Event::EventOrder(order) => {
                            self.on_order(&order).await;
                        },
                        Event::EventEquity(equity) => {
                            self.context.push_equity(equity.clone());
                            self.on_equity(&equity).await;
                        },
                        Event::EventTradeRecord(trade_record) => {
                            self.context.push_trade_record(trade_record.clone());
                            self.on_trade_record(&trade_record).await;
                        },
                    }
                }
                Err(TryRecvError::Empty) => {
                    // No messages available right now, await for new messages
                    let current_time = Instant::now();
                    let duration = current_time.duration_since(start_time);
                    // 判断时间间隔是否超过一分钟
                    if duration > Duration::from_secs(20) {
                        let _ = self.broker.event_sender.send(Event::EventFinish());
                    }
                }
                Err(TryRecvError::Disconnected) => {
                    log::info!("Receiver has closed and no more events will be received.");
                    break;
                }
            }
        }
    }

    pub async fn run(
        &mut self,
    ) {
        let symbols = self.params.symbols.clone();
        let intervals = self.params.intervals.clone();
        let broker = self.broker.clone();
        let _items_timestamp_start = self.params.items_timestamp_start.clone();
        let _items_timestamp_end = self.params.items_timestamp_end.clone();
        self.init().await;
        
    
        let producer_handle = task::spawn(async move {
            broker.start(&symbols, &intervals, &_items_timestamp_start, &_items_timestamp_end).await;
        });
        // 直接调用 handle_events
        self.handle_events().await;
        // 等待 broker 任务完成
        producer_handle.await.expect("Broker task failed");

    }
    async fn init(&mut self) {
        let symbols = self.params.symbols.clone();
        let intervals = self.params.intervals.clone();
        let timestamp = get_timestamp_ms();
        for symbol in symbols {
            for interval in &intervals {
                let item = format!("{}_{}", symbol, interval);
                self.context.push_equity(Equity{
                    item: item.clone(),
                    timestamp,
                    equity_value: self.params.initial_capital,
                    close_latest: 0.0,
                    pos_size: 0.0,
                    cash_aval: self.params.initial_capital,
                });
                self.context.update_position(Position{
                    item: item.clone(),
                    size: 0.0,
                    price: 0.0,
                    highest: 0.0,
                    lowest: 0.0,
                    stop_loss: 0.0,
                    take_profit: 0.0,
                    timestamp,
                });
            }
        }
        self.on_init().await;
    }
    async fn process_order(&mut self, order: &Order) {
        let item = order.item.clone();
        let qty = order.qty;
        let price = order.price;
        let timestamp = order.timestamp;
        if let Some(last_pos) = self.context.get_position(&item) {
            if let Some(value) = self.context.get_atr(&item) {
                let pos_avg_price = (last_pos.size*last_pos.price+qty*price)/(last_pos.size+qty);
                let position = Position{
                    item: item.to_string(),
                    size: if last_pos.size*qty > 0.0 { last_pos.size+qty } else { qty },
                    price: if last_pos.size*qty > 0.0 { pos_avg_price } else { price },
                    highest: if last_pos.highest > 0.0 { last_pos.highest } else { price },
                    lowest: if last_pos.lowest > 0.0 { last_pos.lowest } else { price },
                    stop_loss: pos_avg_price - self.params.n_atr_sl*value,
                    take_profit: pos_avg_price + self.params.n_atr_tp*value,
                    timestamp,
                };
                self.context.update_position(position.clone());
                let _ = self.broker.event_sender.send(Event::EventPosition(position));
            }
        }        
        if let Some(last_trade_record) = self.context.get_last_trade_record(&item) {
            if last_trade_record.size*qty > 0.0 {
                let size = last_trade_record.size + qty;
                let price_open = (last_trade_record.price_open*last_trade_record.size + price*qty)/size;
                let tr = TradeRecord{
                    item: item.to_string(),
                    side: last_trade_record.side.to_string(),
                    size,
                    price_open,
                    time_open: timestamp,
                    price_close: 0.0,
                    time_close: 0,
                    label_close: "".to_string(),
                };
                self.context.update_trade_record(tr);
            } else {
                // close sell or buy
                let tr_close = TradeRecord{
                    item: item.to_string(),
                    side: last_trade_record.side.clone(),
                    size: last_trade_record.size,
                    price_open: last_trade_record.price_open,
                    time_open: last_trade_record.time_open,
                    price_close: price,
                    time_close: timestamp,
                    label_close: "Close".to_string(),
                };
                // update trade record
                self.context.update_trade_record(tr_close);
                // make a new trade record
                let tr = TradeRecord{
                    item: item.to_string(),
                    side: if qty > 0.0 {"buy".to_string()} else {"sell".to_string()},
                    size: qty,
                    price_open: price,
                    time_open: timestamp,
                    price_close: 0.0,
                    time_close: 0,
                    label_close: "".to_string(),
                };
                // push the new trade record
                
                let _ = self.broker.event_sender.send(Event::EventTradeRecord(tr));
            }
            
        } else {
            let tr = TradeRecord{
                item: item.to_string(),
                side: if qty > 0.0 {"buy".to_string()} else {"sell".to_string()},
                size: qty,
                price_open: price,
                time_open: timestamp,
                price_close: 0.0,
                time_close: 0,
                label_close: "".to_string(),
            };
            let _ = self.broker.event_sender.send(Event::EventTradeRecord(tr));
        }
        let _ = self.broker.event_sender.send(Event::EventOrder(order.clone()));
    }
    pub async fn buy(&mut self, item: &String, price: f64, timestamp: i64, qty: Option<f64>) {
        // 判断资金是否够(下单金额和手续费)
        // 够则下单
        // 然后推送on_order
        // 然后推送on_equity
        // 然后推送on_on_position
        // 然后推送on_trade_record
        let qty_value = match qty {
            Some(size) => size,
            None => 0.0,
        };
        let mut margin = if self.params.is_use_percent_of_equity {
            self.params.initial_capital*self.params.percent_of_equity
        } else {
            self.params.initial_capital*self.params.percent_of_every_trade_money
        };
        if 0.0 < qty_value && qty_value < margin {
            margin = qty_value;
        }
        let mut last_equity = Equity{
            item: item.clone(),
            timestamp,
            equity_value: self.params.initial_capital,
            close_latest: 0.0,
            pos_size: 0.0,
            cash_aval: self.params.initial_capital,
        };
        if let Some(last_one) = self.context.get_last_equity(&item) {
            last_equity = last_one.clone();
        }
        if margin*(1.00+self.params.trading_fee) < last_equity.cash_aval {
            let qty = margin/price;
            let order = Order{
                item: item.to_string(),
                price,
                qty,
                timestamp,
            };
            self.process_order(&order).await;
            let equity = Equity{
                item: item.to_string(),
                timestamp,
                equity_value: last_equity.equity_value-margin*self.params.trading_fee,
                close_latest: price,
                pos_size: if last_equity.pos_size > 0.0 { last_equity.pos_size+qty } else { qty },
                cash_aval: last_equity.cash_aval-margin*(1.00+self.params.trading_fee),
            };
            let _ = self.broker.event_sender.send(Event::EventEquity(equity));
        }
        
    }
    pub async fn sell(&mut self, item: &String, price: f64, timestamp: i64, qty: Option<f64>) {
        let qty_value = match qty {
            Some(size) => size,
            None => 0.0,
        };

        let mut margin = if self.params.is_use_percent_of_equity {
            self.params.initial_capital*self.params.percent_of_equity
        } else {
            self.params.initial_capital*self.params.percent_of_every_trade_money
        };
        if 0.0 < qty_value && qty_value < margin {
            margin = qty_value;
        }
        let mut last_equity = Equity{
            item: item.clone(),
            timestamp,
            equity_value: self.params.initial_capital,
            close_latest: 0.0,
            pos_size: 0.0,
            cash_aval: self.params.initial_capital,
        };
        if let Some(last_one) = self.context.get_last_equity(&item) {
            last_equity = last_one.clone();
        }
        if margin*(1.00+self.params.trading_fee) < last_equity.cash_aval {
            let qty = -margin/price;
            let order = Order{
                item: item.to_string(),
                price,
                qty,
                timestamp,
            };
            self.process_order(&order).await;
            let equity = Equity{
                item: item.to_string(),
                timestamp,
                equity_value: last_equity.equity_value-margin*self.params.trading_fee,
                close_latest: price,
                pos_size: if last_equity.pos_size < 0.0 { last_equity.pos_size+qty } else { qty },
                cash_aval: last_equity.cash_aval-margin*(1.00+self.params.trading_fee),
            };
            let _ = self.broker.event_sender.send(Event::EventEquity(equity));
        }
        
    }
}
//####blockcode1 end####
