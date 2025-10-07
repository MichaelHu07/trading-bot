use chrono::NaiveDate;
use serde::Deserialize;


#[derive(Debug, Deserialize, Clone)]
struct OpnHiLoClseVol {
    #[serde(with = "chrono::naive::serde::ts_milliseconds", default)]
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

fn read_csv(path: &str) -> csv::Result<Vec<OpnHiLoClseVol>> {
    let mut rdr = csv::Reader::from_path(path)?;
    let mut rows: Vec<OpnHiLoClseVol> = Vec::new();
    for result in rdr.deserialize() {
        let mut rec: OpnHiLoClseVol = result?;
        rec.date = rec.date.trim().to_string();
        rows.push(rec);
    }
    Ok(rows)
}

fn compute_rsi(closes: &[f64], period: usize) -> Vec<Option<f64>> {
    
}