use chrono::{TimeZone, Utc, DateTime};
pub fn find_max_last_n(vec: &[f64], n: usize) -> f64 {
    if vec.is_empty() || n == 0 || n > vec.len() {
        return -1.00;
    }

    let start = vec.len().saturating_sub(n);
    let mut iter = vec[start..].iter();

    // 初始化最大值和最小值为第一个元素
    let mut max = *iter.next().unwrap();

    // 遍历剩余元素更新最大值和最小值
    for &val in iter {
        if val > max {
            max = val;
        }
    }

    max
}
pub fn find_min_last_n(vec: &[f64], n: usize) -> f64 {
    if vec.is_empty() || n == 0 || n > vec.len() {
        return -1.00;
    }

    let start = vec.len().saturating_sub(n);
    let mut iter = vec[start..].iter();

    // 初始化最大值和最小值为第一个元素
    let mut min = *iter.next().unwrap();

    // 遍历剩余元素更新最大值和最小值
    for &val in iter {
        if val < min {
            min = val;
        }
    }

    min
}

pub fn calculate_sma(prices: &[f64], period: usize) -> Vec<f64> {
    let mut sma = Vec::new();
    for i in 0..prices.len() {
        if i + 1 < period {
            sma.push(f64::NAN); // 不足以计算时填充 NaN
            continue;
        }
        let sum: f64 = prices[i + 1 - period..=i].iter().sum();
        sma.push(sum / period as f64);
    }
    sma
}
pub fn calculate_ema(prices: &[f64], period: usize) -> Vec<f64> {
    let mut ema = Vec::new();
    let multiplier = 2.0 / (period as f64 + 1.0);
    for (i, &price) in prices.iter().enumerate() {
        if i == 0 {
            ema.push(price); // 第一个值设为第一个价格
        } else {
            let prev_ema = ema[i - 1];
            let current_ema = (price - prev_ema) * multiplier + prev_ema;
            ema.push(current_ema);
        }
    }
    ema
}

pub fn timestamp_millis_to_datetime(timestamp_millis: i64) -> DateTime<Utc> {
    match Utc.timestamp_millis_opt(timestamp_millis) {
        chrono::LocalResult::None => panic!("Invalid timestamp"),
        chrono::LocalResult::Single(datetime) => datetime,
        chrono::LocalResult::Ambiguous(_, _) => panic!("Ambiguous timestamp"),
    }
}
