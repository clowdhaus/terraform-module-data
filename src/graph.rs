use std::path::Path;

use anyhow::{Ok, Result};
use chrono::NaiveDate;
use plotly::{
  common::Mode,
  layout::{Axis, Legend, RangeSelector, RangeSlider, SelectorButton, SelectorStep, StepMode},
  Layout, Plot, Scatter,
};

#[derive(Debug)]
pub struct Titles {
  pub title: String,
  pub x_title: String,
  pub y_title: String,
}

pub fn graph(data_path: &Path, _assets_path: &Path) -> Result<()> {
  crate::github::graph(data_path)?;
  crate::registry::graph(data_path)
}

#[derive(Default, Debug)]
pub struct TraceData {
  pub name: String,
  pub x_data: Vec<NaiveDate>,
  pub y_data: Vec<String>,
}

pub(crate) fn plot_time_series(name: &str, data: Vec<TraceData>, titles: Titles, mode: Mode) -> Result<String> {
  let mut plot = Plot::new();

  for d in data.into_iter() {
    let trace = Scatter::new(d.x_data, d.y_data).mode(mode.clone()).name(d.name);
    plot.add_trace(trace);
  }

  let layout = Layout::new()
    .title(&titles.title)
    .legend(Legend::new().title("Module"))
    .height(650)
    .x_axis(
      Axis::new()
        .title(&titles.x_title)
        .range_slider(RangeSlider::new().visible(true))
        .range_selector(RangeSelector::new().buttons(vec![
                        SelectorButton::new()
                            .count(1)
                            .label("1m")
                            .step(SelectorStep::Month)
                            .step_mode(StepMode::Backward),
                        SelectorButton::new()
                            .count(6)
                            .label("6m")
                            .step(SelectorStep::Month)
                            .step_mode(StepMode::Backward),
                        SelectorButton::new()
                            .count(1)
                            .label("YTD")
                            .step(SelectorStep::Year)
                            .step_mode(StepMode::ToDate),
                        SelectorButton::new()
                            .count(1)
                            .label("1y")
                            .step(SelectorStep::Year)
                            .step_mode(StepMode::Backward),
                        SelectorButton::new().step(SelectorStep::All),
                    ])),
    )
    .y_axis(Axis::new().title(&titles.y_title));
  plot.set_layout(layout);

  // plot.show();

  Ok(plot.to_inline_html(Some(name)))
}
