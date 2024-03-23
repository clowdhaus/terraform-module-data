use std::path::Path;

use anyhow::{Ok, Result};
use chrono::NaiveDate;
use plotly::{
  color::Rgb,
  common::{Line, Marker, Mode, Title},
  layout::{Axis, RangeSelector, RangeSlider, SelectorButton, SelectorStep, StepMode},
  Layout, Plot, Scatter,
};

pub struct Titles {
  pub title: String,
  pub x_title: String,
  pub y_title: String,
}

pub fn graph(data_path: &Path, _assets_path: &Path) -> Result<()> {
  crate::github::graph(data_path)?;

  Ok(())
}

pub(crate) fn plot_time_series(x_data: Vec<NaiveDate>, y_data: Vec<String>, titles: Titles) -> Result<()> {
  let trace = Scatter::new(x_data, y_data).mode(Mode::LinesMarkers).marker(
    Marker::new()
      .color(Rgb::new(227, 61, 148))
      .size(4)
      .line(Line::new().color(Rgb::new(227, 61, 148)).width(1.0)),
  );

  let mut plot = Plot::new();
  plot.add_trace(trace);

  let layout = Layout::new()
    .title(Title::new(&titles.title))
    .x_axis(
      Axis::new()
        .title(Title::new(&titles.x_title))
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
    .y_axis(Axis::new().title(Title::new(&titles.y_title)));
  plot.set_layout(layout);

  plot.show();

  println!(
    "{}",
    plot.to_inline_html(Some("time_series_with_range_selector_buttons"))
  );

  Ok(())
}
