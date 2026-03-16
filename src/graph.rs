use std::{fs, path::Path};

use anyhow::Result;
use chrono::{Datelike, NaiveDate};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ChartDataset {
  pub label: String,
  pub data: Vec<DataPoint>,
}

#[derive(Debug, Serialize)]
pub struct DataPoint {
  pub x: String,
  pub y: u64,
}

#[derive(Debug, Serialize)]
pub struct ChartPage {
  pub title: String,
  pub updated_at: String,
  pub sections: Vec<ChartSection>,
}

#[derive(Debug, Serialize)]
pub struct ChartSection {
  pub title: String,
  pub datasets: Vec<ChartDataset>,
}

pub fn graph(data_path: &Path) -> Result<()> {
  let output_path = Path::new("site").join("public").join("data");
  fs::create_dir_all(&output_path)?;

  crate::github::graph(data_path, &output_path)?;
  crate::registry::graph(data_path, &output_path)?;

  Ok(())
}

/// Filter out the current (incomplete) month from date/value pairs
pub fn filter_incomplete_month(dates: Vec<NaiveDate>, values: Vec<u64>) -> (Vec<NaiveDate>, Vec<u64>) {
  let today = chrono::Local::now().date_naive();
  let current_month_start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1);

  match current_month_start {
    Some(cutoff) => {
      let (d, v): (Vec<_>, Vec<_>) = dates.into_iter().zip(values).filter(|(date, _)| *date < cutoff).unzip();
      (d, v)
    }
    None => (dates, values),
  }
}

pub fn write_chart_page(path: &Path, filename: &str, page: &ChartPage) -> Result<()> {
  let json = serde_json::to_string_pretty(page)?;
  fs::write(path.join(filename), json)?;
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_filter_incomplete_month_removes_current() {
    let today = chrono::Local::now().date_naive();
    let current_month = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
    let last_month = current_month - chrono::Months::new(1);

    let dates = vec![last_month, current_month];
    let values = vec![100, 50];

    let (filtered_dates, filtered_values) = filter_incomplete_month(dates, values);
    assert_eq!(filtered_dates.len(), 1);
    assert_eq!(filtered_dates[0], last_month);
    assert_eq!(filtered_values[0], 100);
  }

  #[test]
  fn test_filter_incomplete_month_keeps_complete() {
    let dates = vec![
      NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
      NaiveDate::from_ymd_opt(2024, 2, 1).unwrap(),
    ];
    let values = vec![100, 200];

    let (filtered_dates, filtered_values) = filter_incomplete_month(dates, values);
    assert_eq!(filtered_dates.len(), 2);
    assert_eq!(filtered_values, vec![100, 200]);
  }
}
