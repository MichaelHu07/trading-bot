use chrono::NaiveDate;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
struct Ohlcv {
    #[serde(with = "chrono::naive::serde::ts_seconds_option", default)]
    #[serde(skip_deserializing)]
    _ts: Option<NaiveDate>,
    #[serde(rename = "date")]
    date: String,
    #[serde(rename = "open")]
    open: f64,
    #[serde(rename = "high")]
    high: f64,
    #[serde(rename = "low")]
    low: f64,
    #[serde(rename = "close")]
    close: f64,
    #[serde(rename = "volume")]
    volume: f64,
}

fn read_csv(path: &str) -> csv::Result<Vec<Ohlcv>> {
    let mut rdr = csv::Reader::from_path(path)?;
    let mut rows: Vec<Ohlcv> = Vec::new();
    for result in rdr.deserialize() {
        let mut rec: Ohlcv = result?;
        rec.date = rec.date.trim().to_string();
        rows.push(rec);
    }
    Ok(rows)
}

fn compute_rsi(closes: &[f64], period: usize) -> Vec<Option<f64>> {
    if closes.len() < period + 1 || period == 0 { return vec![None; closes.len()]; }
    let mut rsis: Vec<Option<f64>> = vec![None; closes.len()];
    let mut gains = 0.0;
    let mut losses = 0.0;
    for i in 1..=period { // first window (1..=period) uses closes[0..=period]
        let change = closes[i] - closes[i - 1];
        if change >= 0.0 { gains += change; } else { losses -= change; }
    }
    let mut avg_gain = gains / period as f64;
    let mut avg_loss = losses / period as f64;
    let rs = if avg_loss == 0.0 { f64::INFINITY } else { avg_gain / avg_loss };
    rsis[period] = Some(100.0 - 100.0 / (1.0 + rs));
    for i in (period + 1)..closes.len() {
        let change = closes[i] - closes[i - 1];
        let gain = change.max(0.0);
        let loss = (-change).max(0.0);
        avg_gain = (avg_gain * (period as f64 - 1.0) + gain) / period as f64;
        avg_loss = (avg_loss * (period as f64 - 1.0) + loss) / period as f64;
        let rs = if avg_loss == 0.0 { f64::INFINITY } else { avg_gain / avg_loss };
        rsis[i] = Some(100.0 - 100.0 / (1.0 + rs));
    }
    rsis
}

fn volume_relative_high(volumes: &[f64], window: usize) -> Vec<bool> {
    if volumes.is_empty() { return vec![]; }
    let mut res = vec![false; volumes.len()];
    for i in 0..volumes.len() {
        if i < window { continue; }
        let slice = &volumes[i - window..i];
        let max_prev = slice
            .iter()
            .fold(f64::MIN, |acc, v| if *v > acc { *v } else { acc });
        res[i] = volumes[i] > max_prev;
    }
    res
}

#[derive(Debug, Clone)]
struct IpoInfo {
    symbol: String,
    lockup_expiration_date: NaiveDate,
}

fn ipo_lockup_screener_stub(today: NaiveDate) -> Vec<IpoInfo> {
    // Placeholder: In real usage, fetch IPO and lockup data from API.
    // Here we just return an empty list or a hardcoded example for demo.
    let example = IpoInfo {
        symbol: "DEMO".to_string(),
        lockup_expiration_date: today, // treat as expiring today
    };
    vec![example]
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum PositionSide { Short, Flat }

#[derive(Debug, Clone)]
struct Trade {
    entry_price: f64,
    exit_price: Option<f64>,
    quantity: f64,
    entry_index: usize,
    exit_index: Option<usize>,
}

#[derive(Debug, Default)]
struct BacktestResult {
    trades: Vec<Trade>,
    total_pnl: f64,
    wins: usize,
    losses: usize,
}

fn run_strategy(ohlcv: &[Ohlcv], symbol: &str) -> BacktestResult {
    if ohlcv.is_empty() { return BacktestResult::default(); }
    let closes: Vec<f64> = ohlcv.iter().map(|r| r.close).collect();
    let volumes: Vec<f64> = ohlcv.iter().map(|r| r.volume).collect();
    let rsi = compute_rsi(&closes, 14);
    let vol_high = volume_relative_high(&volumes, 20);

    let mut result = BacktestResult::default();
    let mut current: Option<Trade> = None;

    for i in 0..ohlcv.len() {
        let date = &ohlcv[i].date;
        let today = NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970,1,1).unwrap());
        let ipos = ipo_lockup_screener_stub(today);
        let within_lockup_window = ipos.iter().any(|_ipo| {
            // In a real screener compare symbol matches and days between today and lockup date in 1..=3
            true
        });

        let rsi_ok = rsi[i].map(|v| v > 65.0).unwrap_or(false);
        let vol_ok = vol_high[i];

        // Entry condition: RSI > 65, volume at relative high, IPO lockup 1-3 days (stubbed)
        if current.is_none() && rsi_ok && vol_ok && within_lockup_window {
            current = Some(Trade { entry_price: closes[i], exit_price: None, quantity: 1.0, entry_index: i, exit_index: None });
        }

        // Exit condition: RSI crosses back below 55 or simple take-profit/stop-loss
        if let Some(tr) = &mut current {
            let rsi_val = rsi[i];
            let take_profit = tr.entry_price * 0.97; // 3% move in favor for short
            let stop_loss = tr.entry_price * 1.03;    // 3% adverse move
            let price = closes[i];
            let exit_signal = rsi_val.map(|v| v < 55.0).unwrap_or(false) || price <= take_profit || price >= stop_loss;
            if exit_signal {
                tr.exit_price = Some(price);
                tr.exit_index = Some(i);
                let pnl = (tr.entry_price - price) * tr.quantity; // short PnL
                result.total_pnl += pnl;
                if pnl >= 0.0 { result.wins += 1; } else { result.losses += 1; }
                result.trades.push(tr.clone());
                current = None;
            }
        }
    }

    // If position left open, close at last price
    if let Some(mut tr) = current {
        let last_price = *closes.last().unwrap();
        tr.exit_price = Some(last_price);
        tr.exit_index = Some(ohlcv.len() - 1);
        let pnl = (tr.entry_price - last_price) * tr.quantity;
        result.total_pnl += pnl;
        if pnl >= 0.0 { result.wins += 1; } else { result.losses += 1; }
        result.trades.push(tr);
    }

    println!("{}: trades={}, pnl={:.2}, wins={}, losses={}", symbol, result.trades.len(), result.total_pnl, result.wins, result.losses);
    result
}

fn main() {
    // Example usage: load CSV with columns: date,open,high,low,close,volume
    let path = "data/sample.csv";
    match read_csv(path) {
        Ok(rows) => {
            if rows.is_empty() {
                println!("No data found in {}", path);
                return;
            }
            let _ = run_strategy(&rows, "DEMO");
        }
        Err(e) => {
            println!("Failed to read {}: {}", path, e);
        }
    }
}
