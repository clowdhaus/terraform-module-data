# Astro + Chart.js Migration Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace mdBook + Plotly with an Astro static site using Chart.js for interactive charts.

**Architecture:** Rust code outputs JSON data files (no more inline HTML generation). Astro site in `site/` loads JSON at build time and renders Chart.js charts client-side. Incomplete months are filtered out to prevent data skew.

**Tech Stack:** Astro 5, Chart.js 4.5, chartjs-adapter-date-fns, chartjs-plugin-zoom, catppuccin theme colors

---

### Task 1: Rust — Replace Plotly with JSON Output

**Goal:** Rewrite `graph.rs`, the graph functions in `github.rs` and `registry.rs` to output JSON data files instead of Plotly HTML. Filter out incomplete (current) months.

**Files:**
- Modify: `Cargo.toml` (remove `plotly` dependency)
- Rewrite: `src/graph.rs`
- Modify: `src/github.rs` (lines 125-243 — graph functions)
- Modify: `src/registry.rs` (lines 150-266 — graph functions)

**Step 1: Remove plotly from Cargo.toml**

In `Cargo.toml`, remove the line:
```
plotly = "0.14"
```

**Step 2: Rewrite `src/graph.rs`**

Replace entire file with:

```rust
use std::{fs, path::Path};

use anyhow::Result;
use chrono::NaiveDate;
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
      let (d, v): (Vec<_>, Vec<_>) = dates
        .into_iter()
        .zip(values)
        .filter(|(date, _)| *date < cutoff)
        .unzip();
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
  use chrono::Datelike;

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
```

**Step 3: Rewrite graph functions in `src/github.rs`**

Replace everything from line 124 (`/// Graph the data...`) to end of file (before `#[cfg(test)]` if any) with:

```rust
/// Output JSON data for the Astro site
pub(crate) fn graph(data_path: &Path, output_path: &Path) -> Result<()> {
  let timestamp = chrono::Local::now().to_utc().format("%Y-%m-%d %H:%M:%S").to_string();

  write_traffic_json(&timestamp, data_path, output_path, "Repository Clones", "clones", "github-clones.json")?;
  write_traffic_json(&timestamp, data_path, output_path, "Repository Page Views", "views", "github-views.json")?;

  Ok(())
}

fn write_traffic_json(
  timestamp: &str,
  data_path: &Path,
  output_path: &Path,
  title: &str,
  data_type: &str,
  filename: &str,
) -> Result<()> {
  let category_names: &[(&str, Option<&str>)] = &[
    ("All", None),
    ("Compute", Some(crate::COMPUTE)),
    ("Serverless", Some(crate::SERVERLESS)),
    ("Data", Some(crate::DATA)),
    ("Networking", Some(crate::NETWORKING)),
    ("Other", Some(crate::OTHER)),
  ];

  let mut sections = Vec::new();
  for &(section_title, category) in category_names {
    let datasets = collect_traffic_datasets(category, data_type, data_path)?;
    sections.push(crate::graph::ChartSection {
      title: section_title.to_string(),
      datasets,
    });
  }

  let page = crate::graph::ChartPage {
    title: title.to_string(),
    updated_at: timestamp.to_string(),
    sections,
  };

  info!("Writing {filename}");
  crate::graph::write_chart_page(output_path, filename, &page)
}

fn collect_traffic_datasets(
  category: Option<&str>,
  data_type: &str,
  data_path: &Path,
) -> Result<Vec<crate::graph::ChartDataset>> {
  let mut datasets = Vec::new();

  for entry in fs::read_dir(data_path.join("github"))? {
    let entry = entry?;
    let dir_name = entry
      .path()
      .file_stem()
      .ok_or_else(|| anyhow::anyhow!("Missing file stem for path: {:?}", entry.path()))?
      .to_str()
      .ok_or_else(|| anyhow::anyhow!("Non-UTF8 file stem for path: {:?}", entry.path()))?
      .to_owned();
    let filepath = entry.path().join(format!("{data_type}.json"));

    // If directory is not in category, skip; if no category provided, return all
    if let Some(c) = category
      && !crate::CATEGORIES
        .get(c)
        .ok_or_else(|| anyhow::anyhow!("Unknown category: {c}"))?
        .contains(&dir_name.as_str())
    {
      continue;
    }

    let data = fs::read_to_string(filepath)?;
    let summary: TrafficSummary = serde_json::from_str(&data)?;

    // Aggregate daily data into monthly buckets (sum counts per month)
    let mut monthly: BTreeMap<chrono::NaiveDate, u64> = BTreeMap::new();
    for (_, v) in summary.iter() {
      let ts = chrono::DateTime::parse_from_rfc3339(&v.timestamp).context("Failed to parse timestamp")?;
      let date = ts.date_naive();
      let month_start = chrono::NaiveDate::from_ymd_opt(date.year(), date.month(), 1)
        .ok_or_else(|| anyhow::anyhow!("Invalid date: {date}"))?;
      *monthly.entry(month_start).or_insert(0) += v.count;
    }

    let dates: Vec<chrono::NaiveDate> = monthly.keys().copied().collect();
    let values: Vec<u64> = monthly.values().copied().collect();
    let (dates, values) = crate::graph::filter_incomplete_month(dates, values);

    let data_points = dates
      .iter()
      .zip(values.iter())
      .map(|(d, v)| crate::graph::DataPoint {
        x: d.to_string(),
        y: *v,
      })
      .collect();

    datasets.push(crate::graph::ChartDataset {
      label: dir_name,
      data: data_points,
    });
  }

  // Sort datasets by label for consistent output
  datasets.sort_by(|a, b| a.label.cmp(&b.label));

  Ok(datasets)
}
```

Also update the imports at the top of `src/github.rs`. Remove `PathBuf` from imports (it's no longer used in graph functions). Keep the existing `use` block but ensure `BTreeMap`, `fs`, `Path` are imported (they already are).

**Step 4: Rewrite graph functions in `src/registry.rs`**

Replace everything from the `type Module` line (150) through the `graph` function (line 266, before `#[cfg(test)]`) with:

```rust
type Module = String;
type ModuleData = BTreeMap<Module, Vec<VersionTrace>>;

#[derive(Debug)]
struct VersionTrace {
  name: String,
  dates: Vec<NaiveDate>,
  values: Vec<u64>,
}

fn collect_trace_data(data_path: &Path) -> Result<ModuleData> {
  let mut data = ModuleData::new();

  for entry in fs::read_dir(data_path.join("registry"))? {
    let mod_path = entry?.path();
    let module = mod_path
      .file_stem()
      .ok_or_else(|| anyhow::anyhow!("Missing file stem for path: {:?}", mod_path))?
      .to_str()
      .ok_or_else(|| anyhow::anyhow!("Non-UTF8 file stem for path: {:?}", mod_path))?
      .to_owned();

    let traces = get_module_data_traces(&mod_path)?;
    data.insert(module, traces);
  }

  Ok(data)
}

fn get_module_data_traces(mod_path: &Path) -> Result<Vec<VersionTrace>> {
  let module_name = mod_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

  let mut daily: BTreeMap<String, BTreeMap<NaiveDate, u64>> = BTreeMap::new();

  for fentry in fs::read_dir(mod_path)? {
    let file_path = fentry?.path();
    let file_name = file_path
      .file_stem()
      .ok_or_else(|| anyhow::anyhow!("Missing file stem for path: {:?}", file_path))?
      .to_str()
      .ok_or_else(|| anyhow::anyhow!("Non-UTF8 file stem for path: {:?}", file_path))?;
    let file_data = fs::read_to_string(&file_path)?;
    let summary = serde_json::from_str::<Vec<Summary>>(&file_data)?;

    let timestamp = NaiveDate::parse_from_str(file_name, "%Y-%m-%d")?;

    for sum in summary.iter() {
      daily
        .entry(sum.major_version.clone())
        .or_default()
        .insert(timestamp, sum.downloads);
    }
  }

  let mut traces = Vec::new();
  for (version, date_values) in daily.into_iter() {
    if module_name == "eks" && version.parse::<i32>().unwrap_or(0) < 16 {
      tracing::debug!("Skipping {mod_path:#?} version {version} (pre-v16 EKS)");
      continue;
    }

    let mut monthly: BTreeMap<NaiveDate, u64> = BTreeMap::new();
    for (date, count) in date_values {
      let month_start =
        NaiveDate::from_ymd_opt(date.year(), date.month(), 1).ok_or_else(|| anyhow::anyhow!("Invalid date: {date}"))?;
      monthly.insert(month_start, count);
    }

    let dates: Vec<NaiveDate> = monthly.keys().copied().collect();
    let values: Vec<u64> = monthly.values().copied().collect();
    let (dates, values) = crate::graph::filter_incomplete_month(dates, values);

    traces.push(VersionTrace {
      name: format!("v{version}.0"),
      dates,
      values,
    });
  }

  Ok(traces)
}

/// Output JSON data for the Astro site
pub(crate) fn graph(data_path: &Path, output_path: &Path) -> Result<()> {
  let timestamp = chrono::Local::now().to_utc().format("%Y-%m-%d %H:%M:%S").to_string();

  let title = "Terraform Registry Downloads";
  let tdata = collect_trace_data(data_path)?;

  let mut sections = Vec::new();
  for (module, traces) in tdata.into_iter() {
    let datasets = traces
      .into_iter()
      .map(|t| {
        let data_points = t
          .dates
          .iter()
          .zip(t.values.iter())
          .map(|(d, v)| crate::graph::DataPoint {
            x: d.to_string(),
            y: *v,
          })
          .collect();
        crate::graph::ChartDataset {
          label: t.name,
          data: data_points,
        }
      })
      .collect();

    sections.push(crate::graph::ChartSection {
      title: module,
      datasets,
    });
  }

  let page = crate::graph::ChartPage {
    title: title.to_string(),
    updated_at: timestamp,
    sections,
  };

  info!("Writing registry-downloads.json");
  crate::graph::write_chart_page(output_path, "registry-downloads.json", &page)
}
```

Also remove the now-unused import of `PathBuf` from `src/registry.rs` if it's only used in the old graph functions. Remove the `Datelike` import if present (it was removed earlier). The `use chrono::prelude::*` at top already provides what's needed.

**Step 5: Remove old `TraceData` references from `src/registry.rs`**

The old code referenced `crate::graph::TraceData`. This type no longer exists. The new code uses the local `VersionTrace` struct instead.

**Step 6: Update `src/graph.rs` public API**

The old `graph.rs` exported `TraceData`, `Titles`, and `plot_time_series`. Remove all references:
- `src/github.rs` no longer uses `crate::graph::TraceData` or `crate::graph::Titles`
- `src/registry.rs` no longer uses `crate::graph::TraceData` or `crate::graph::Titles`

**Step 7: Run `cargo build` and fix any compilation errors**

Run: `cargo build 2>&1`
Expected: Successful compilation (no plotly references remain)

**Step 8: Run `cargo test`**

Run: `cargo test 2>&1`
Expected: All tests pass (existing tests + new `filter_incomplete_month` tests)

**Step 9: Run `cargo run -- graph` to generate JSON files**

Run: `cargo run -- graph 2>&1`
Expected: Creates `site/public/data/github-clones.json`, `site/public/data/github-views.json`, `site/public/data/registry-downloads.json`

Verify: `ls site/public/data/` shows 3 JSON files. Quick-check one:
```bash
cat site/public/data/github-clones.json | python3 -m json.tool | head -20
```

**Step 10: Commit**

```bash
git add Cargo.toml Cargo.lock src/graph.rs src/github.rs src/registry.rs
git commit -m "refactor: replace plotly with JSON data output for Astro migration"
```

---

### Task 2: Scaffold Astro Site

**Goal:** Create the Astro project in `site/` with Chart.js, catppuccin theme, and pages for each chart type.

**Files:**
- Create: `site/package.json`
- Create: `site/astro.config.mjs`
- Create: `site/tsconfig.json`
- Create: `site/src/layouts/Base.astro`
- Create: `site/src/components/Nav.astro`
- Create: `site/src/components/Chart.astro`
- Create: `site/src/styles/global.css`
- Create: `site/src/pages/index.astro`
- Create: `site/src/pages/github-clones.astro`
- Create: `site/src/pages/github-views.astro`
- Create: `site/src/pages/registry.astro`

**Step 1: Initialize Astro project**

```bash
cd site
npm create astro@latest . -- --template minimal --no-git --no-install --typescript strict
npm install chart.js chartjs-adapter-date-fns chartjs-plugin-zoom
cd ..
```

**Step 2: Create `site/astro.config.mjs`**

```javascript
import { defineConfig } from 'astro/config';

export default defineConfig({
  site: 'https://clowdhaus.github.io',
  base: '/terraform-module-data',
});
```

**Step 3: Create `site/src/styles/global.css`**

Use catppuccin mocha palette:

```css
:root {
  --ctp-base: #1e1e2e;
  --ctp-mantle: #181825;
  --ctp-crust: #11111b;
  --ctp-surface0: #313244;
  --ctp-surface1: #45475a;
  --ctp-surface2: #585b70;
  --ctp-overlay0: #6c7086;
  --ctp-overlay1: #7f849c;
  --ctp-text: #cdd6f4;
  --ctp-subtext0: #a6adc8;
  --ctp-subtext1: #bac2de;
  --ctp-blue: #89b4fa;
  --ctp-green: #a6e3a1;
  --ctp-red: #f38ba8;
  --ctp-peach: #fab387;
  --ctp-mauve: #cba6f7;
  --ctp-yellow: #f9e2af;
  --ctp-teal: #94e2d5;
  --ctp-sky: #89dceb;
  --ctp-pink: #f5c2e7;
  --ctp-flamingo: #f2cdcd;
  --ctp-rosewater: #f5e0dc;
  --ctp-lavender: #b4befe;
  --ctp-sapphire: #74c7ec;
  --ctp-maroon: #eba0ac;
}

* { margin: 0; padding: 0; box-sizing: border-box; }

body {
  font-family: system-ui, -apple-system, 'Segoe UI', sans-serif;
  background: var(--ctp-base);
  color: var(--ctp-text);
  line-height: 1.6;
}

a {
  color: var(--ctp-blue);
  text-decoration: none;
}

a:hover {
  text-decoration: underline;
}

.container {
  max-width: 1200px;
  margin: 0 auto;
  padding: 0 1.5rem;
}

.chart-container {
  background: var(--ctp-mantle);
  border-radius: 8px;
  border: 1px solid var(--ctp-surface0);
  padding: 1.5rem;
  margin-bottom: 2rem;
}

.chart-container h3 {
  color: var(--ctp-subtext1);
  margin-bottom: 1rem;
  font-size: 1.1rem;
}

.chart-container canvas {
  width: 100% !important;
}

.updated-at {
  color: var(--ctp-overlay0);
  font-size: 0.875rem;
  margin-bottom: 2rem;
}
```

**Step 4: Create `site/src/components/Nav.astro`**

```astro
---
const currentPath = Astro.url.pathname;
const base = import.meta.env.BASE_URL;

const links = [
  { href: `${base}/`, label: 'Overview' },
  { href: `${base}/github-clones/`, label: 'GitHub Clones' },
  { href: `${base}/github-views/`, label: 'GitHub Views' },
  { href: `${base}/registry/`, label: 'Registry Downloads' },
];
---

<nav>
  <div class="nav-inner container">
    <a href={`${base}/`} class="nav-brand">Terraform Module Data</a>
    <div class="nav-links">
      {links.map(link => (
        <a
          href={link.href}
          class:list={['nav-link', { active: currentPath === link.href }]}
        >
          {link.label}
        </a>
      ))}
    </div>
  </div>
</nav>

<style>
  nav {
    background: var(--ctp-mantle);
    border-bottom: 1px solid var(--ctp-surface0);
    padding: 0.75rem 0;
    position: sticky;
    top: 0;
    z-index: 100;
  }
  .nav-inner {
    display: flex;
    align-items: center;
    gap: 2rem;
  }
  .nav-brand {
    font-weight: 700;
    font-size: 1.1rem;
    color: var(--ctp-text);
  }
  .nav-brand:hover { text-decoration: none; }
  .nav-links {
    display: flex;
    gap: 1rem;
  }
  .nav-link {
    color: var(--ctp-overlay1);
    font-size: 0.9rem;
    padding: 0.25rem 0.5rem;
    border-radius: 4px;
    transition: color 0.15s, background 0.15s;
  }
  .nav-link:hover {
    color: var(--ctp-text);
    background: var(--ctp-surface0);
    text-decoration: none;
  }
  .nav-link.active {
    color: var(--ctp-blue);
    background: var(--ctp-surface0);
  }
</style>
```

**Step 5: Create `site/src/layouts/Base.astro`**

```astro
---
import Nav from '../components/Nav.astro';
import '../styles/global.css';

interface Props {
  title: string;
}

const { title } = Astro.props;
---

<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>{title} - Terraform Module Data</title>
    <link rel="icon" type="image/svg+xml" href="/favicon.svg" />
  </head>
  <body>
    <Nav />
    <main class="container">
      <slot />
    </main>
  </body>
</html>

<style>
  main {
    padding: 2rem 1.5rem;
  }
</style>
```

**Step 6: Create `site/src/components/Chart.astro`**

This component renders a single Chart.js chart from JSON datasets.

```astro
---
interface DataPoint {
  x: string;
  y: number;
}

interface Dataset {
  label: string;
  data: DataPoint[];
}

interface Props {
  id: string;
  title: string;
  datasets: Dataset[];
  yLabel?: string;
  mode?: 'lines' | 'markers';
}

const { id, title, datasets, yLabel = 'Count', mode = 'lines' } = Astro.props;
---

<div class="chart-container">
  <h3>{title}</h3>
  <canvas
    id={id}
    data-datasets={JSON.stringify(datasets)}
    data-y-label={yLabel}
    data-mode={mode}
  ></canvas>
</div>

<script>
  import Chart from 'chart.js/auto';
  import 'chartjs-adapter-date-fns';
  import zoomPlugin from 'chartjs-plugin-zoom';

  Chart.register(zoomPlugin);

  const COLORS = [
    '#89b4fa', '#a6e3a1', '#f38ba8', '#fab387', '#cba6f7',
    '#f9e2af', '#94e2d5', '#89dceb', '#f5c2e7', '#f2cdcd',
    '#f5e0dc', '#b4befe', '#74c7ec', '#eba0ac',
  ];

  function initChart(canvas: HTMLCanvasElement) {
    const datasets = JSON.parse(canvas.dataset.datasets || '[]');
    const yLabel = canvas.dataset.yLabel || 'Count';
    const mode = canvas.dataset.mode || 'lines';

    const chartDatasets = datasets.map((ds: any, i: number) => ({
      label: ds.label,
      data: ds.data.map((p: any) => ({ x: p.x, y: p.y })),
      borderColor: COLORS[i % COLORS.length],
      backgroundColor: COLORS[i % COLORS.length] + '20',
      borderWidth: 2,
      pointRadius: mode === 'markers' ? 4 : 2,
      pointHoverRadius: 6,
      tension: 0.1,
    }));

    new Chart(canvas, {
      type: 'line',
      data: { datasets: chartDatasets },
      options: {
        responsive: true,
        maintainAspectRatio: true,
        aspectRatio: 2.5,
        interaction: { mode: 'index', intersect: false },
        scales: {
          x: {
            type: 'time',
            time: { unit: 'month', tooltipFormat: 'MMM yyyy' },
            grid: { color: '#31324420' },
            ticks: { color: '#6c7086' },
          },
          y: {
            title: { display: true, text: yLabel, color: '#a6adc8' },
            grid: { color: '#31324420' },
            ticks: {
              color: '#6c7086',
              callback: function(value: number | string) {
                const v = Number(value);
                if (v >= 1e6) return (v / 1e6).toFixed(1) + 'M';
                if (v >= 1e3) return (v / 1e3).toFixed(0) + 'K';
                return v.toString();
              },
            },
          },
        },
        plugins: {
          legend: {
            labels: { color: '#cdd6f4', usePointStyle: true, pointStyle: 'circle' },
          },
          zoom: {
            pan: { enabled: true, mode: 'x' },
            zoom: {
              wheel: { enabled: true },
              pinch: { enabled: true },
              drag: {
                enabled: true,
                backgroundColor: 'rgba(137,180,250,0.1)',
                borderColor: '#89b4fa',
              },
              mode: 'x',
            },
          },
        },
      },
    });
  }

  // Initialize all chart canvases on the page
  document.querySelectorAll<HTMLCanvasElement>('canvas[data-datasets]').forEach(initChart);
</script>
```

**Step 7: Create `site/src/pages/index.astro`**

```astro
---
import Base from '../layouts/Base.astro';
---

<Base title="Overview">
  <h1>Terraform Module Data</h1>
  <p class="subtitle">
    Download and traffic analytics for
    <a href="https://github.com/terraform-aws-modules">terraform-aws-modules</a>.
  </p>

  <div class="cards">
    <a href={`${import.meta.env.BASE_URL}/github-clones/`} class="card">
      <h2>GitHub Clones</h2>
      <p>Repository clone traffic over time</p>
    </a>
    <a href={`${import.meta.env.BASE_URL}/github-views/`} class="card">
      <h2>GitHub Views</h2>
      <p>Repository page view traffic over time</p>
    </a>
    <a href={`${import.meta.env.BASE_URL}/registry/`} class="card">
      <h2>Registry Downloads</h2>
      <p>Terraform Registry download counts by version</p>
    </a>
  </div>
</Base>

<style>
  h1 { margin-bottom: 0.5rem; }
  .subtitle {
    color: var(--ctp-subtext0);
    margin-bottom: 2rem;
    font-size: 1.1rem;
  }
  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 1rem;
  }
  .card {
    background: var(--ctp-mantle);
    border: 1px solid var(--ctp-surface0);
    border-radius: 8px;
    padding: 1.5rem;
    transition: border-color 0.15s;
    color: var(--ctp-text);
  }
  .card:hover {
    border-color: var(--ctp-blue);
    text-decoration: none;
  }
  .card h2 {
    font-size: 1.2rem;
    margin-bottom: 0.5rem;
    color: var(--ctp-blue);
  }
  .card p { color: var(--ctp-subtext0); font-size: 0.9rem; }
</style>
```

**Step 8: Create `site/src/pages/github-clones.astro`**

```astro
---
import Base from '../layouts/Base.astro';
import Chart from '../components/Chart.astro';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const dataPath = join(process.cwd(), 'public', 'data', 'github-clones.json');
const page = JSON.parse(readFileSync(dataPath, 'utf-8'));
---

<Base title="GitHub Clones">
  <h1>{page.title}</h1>
  <p class="updated-at">Last updated: {page.updated_at} UTC</p>

  {page.sections.map((section: any) => (
    <Chart
      id={`chart-${section.title.toLowerCase().replace(/\s+/g, '-')}`}
      title={section.title}
      datasets={section.datasets}
      yLabel="Clones"
    />
  ))}
</Base>
```

**Step 9: Create `site/src/pages/github-views.astro`**

```astro
---
import Base from '../layouts/Base.astro';
import Chart from '../components/Chart.astro';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const dataPath = join(process.cwd(), 'public', 'data', 'github-views.json');
const page = JSON.parse(readFileSync(dataPath, 'utf-8'));
---

<Base title="GitHub Page Views">
  <h1>{page.title}</h1>
  <p class="updated-at">Last updated: {page.updated_at} UTC</p>

  {page.sections.map((section: any) => (
    <Chart
      id={`chart-${section.title.toLowerCase().replace(/\s+/g, '-')}`}
      title={section.title}
      datasets={section.datasets}
      yLabel="Views"
    />
  ))}
</Base>
```

**Step 10: Create `site/src/pages/registry.astro`**

```astro
---
import Base from '../layouts/Base.astro';
import Chart from '../components/Chart.astro';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const dataPath = join(process.cwd(), 'public', 'data', 'registry-downloads.json');
const page = JSON.parse(readFileSync(dataPath, 'utf-8'));
---

<Base title="Registry Downloads">
  <h1>{page.title}</h1>
  <p class="updated-at">Last updated: {page.updated_at} UTC</p>

  {page.sections.map((section: any) => (
    <Chart
      id={`chart-${section.title.toLowerCase().replace(/\s+/g, '-')}`}
      title={section.title}
      datasets={section.datasets}
      yLabel="Downloads"
      mode="markers"
    />
  ))}
</Base>
```

**Step 11: Build and test locally**

```bash
# Generate JSON data
cargo run -- graph

# Build Astro site
cd site && npm run build && cd ..

# Preview locally
cd site && npm run preview && cd ..
```

Open in browser and verify all 3 pages render charts correctly with zoom/pan.

**Step 12: Commit**

```bash
git add site/
git commit -m "feat: add Astro site with Chart.js charts"
```

---

### Task 3: Delete mdBook Files

**Goal:** Remove all mdBook-related files and config.

**Files:**
- Delete: `book.toml`
- Delete: `theme/` (entire directory)
- Delete: `docs/SUMMARY.md`
- Delete: `docs/github-clones.md`
- Delete: `docs/github-page-views.md`
- Delete: `docs/registry-downloads.md`
- Delete: `docs/assets/` (entire directory)
- Delete: `src/templates/` (entire directory)
- Delete: `compare-charts.html` (temporary comparison file)

**Step 1: Remove files**

```bash
rm -rf book.toml theme/ docs/SUMMARY.md docs/github-clones.md docs/github-page-views.md docs/registry-downloads.md docs/assets/ src/templates/ compare-charts.html
```

**Step 2: Verify nothing references deleted files**

```bash
grep -r "book.toml\|mdbook\|mdBook\|plotly" src/ --include="*.rs" || echo "No stale references"
grep -r "templates/" src/ --include="*.rs" || echo "No template references"
```

Expected: No matches (or only this plan file and comments)

**Step 3: Commit**

```bash
git add -A
git commit -m "chore: remove mdBook files and templates"
```

---

### Task 4: Update CI Workflows

**Goal:** Update the docs workflow to build Astro instead of mdBook. Update the daily update workflow to generate JSON before committing.

**Files:**
- Rewrite: `.github/workflows/docs.yaml`
- Modify: `.github/workflows/update.yaml`
- Modify: `.gitignore` (add site/public/data/ and site build output)
- Modify: `.gitattributes` (update linguist-generated patterns)

**Step 1: Update `.gitignore`**

Add to `.gitignore`:
```
site/node_modules/
site/dist/
site/.astro/
```

Note: Do NOT gitignore `site/public/data/` — the JSON files should be committed so the site can be built without running the Rust binary.

**Step 2: Update `.gitattributes`**

Replace the old mdBook generated file patterns with the new JSON data patterns:

```
site/public/data/*.json linguist-generated=true
```

Remove old patterns referencing `docs/github-clones.md`, `docs/github-page-views.md`, `docs/registry-downloads.md`.

**Step 3: Rewrite `.github/workflows/docs.yaml`**

```yaml
name: Publish docs

on:
  push:
    branches:
      - main
  workflow_dispatch:

permissions:
  contents: read
  pages: write
  id-token: write

concurrency:
  group: "pages"
  cancel-in-progress: false

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@de0fac2e4500dabe0009e67214ff5f5447ce83dd # v6

      - name: Setup Node.js
        uses: actions/setup-node@49933ea5288caeca8642d1e84afbd3f7d6820020 # v4
        with:
          node-version: 22
          cache: npm
          cache-dependency-path: site/package-lock.json

      - name: Install dependencies
        run: npm ci
        working-directory: site

      - name: Build Astro site
        run: npm run build
        working-directory: site

      - name: Setup Pages
        id: pages
        uses: actions/configure-pages@983d7736d9b0ae728b81ab479565c72886d7745b # v5

      - name: Upload artifact
        uses: actions/upload-pages-artifact@7b1f4a764d45c48632c6b24a0339c27f5614fb0b # v4
        with:
          path: site/dist

  deploy:
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    runs-on: ubuntu-latest
    needs: build
    steps:
      - name: Deploy to GitHub Pages
        id: deployment
        uses: actions/deploy-pages@d6db90164ac5ed86f2b6aed7e0febac5b3c0c03e # v4
```

**Step 4: Update `.github/workflows/update.yaml`**

The `collect.sh` script already runs `target/release/tmd graph` which now outputs JSON to `site/public/data/`. The generated JSON files are committed alongside the raw data. No changes needed to the workflow itself, but verify `collect.sh` still works:

The existing `collect.sh` line `target/release/tmd graph` will now write to `site/public/data/` instead of `docs/`. The `git add -A` in the EndBug/add-and-commit action will pick up the new JSON files. No changes needed.

**Step 5: Verify the SHA pins for `actions/setup-node`**

Run:
```bash
gh api repos/actions/setup-node/git/refs/tags/v4 --jq '.object.sha'
```
Use the returned SHA. The one in the plan (`49933ea5288caeca8642d1e84afbd3f7d6820020`) should be verified.

**Step 6: Commit**

```bash
git add .github/workflows/docs.yaml .github/workflows/update.yaml .gitignore .gitattributes
git commit -m "ci: update workflows for Astro site deployment"
```

---

### Task 5: Local Verification

**Goal:** Full end-to-end test before pushing.

**Step 1: Build Rust and generate JSON**

```bash
cargo build --release
cargo run -- graph
ls -la site/public/data/
```

Expected: 3 JSON files exist with reasonable sizes.

**Step 2: Build Astro site**

```bash
cd site && npm run build && cd ..
```

Expected: `site/dist/` contains `index.html`, `github-clones/index.html`, `github-views/index.html`, `registry/index.html` plus JS/CSS assets.

**Step 3: Preview and verify**

```bash
cd site && npm run preview
```

Open in browser. Verify:
- All 4 pages load
- Charts render with correct data
- Zoom/pan works (drag, scroll wheel, double-click reset)
- No incomplete current month data point
- Navigation works between pages
- Catppuccin dark theme looks correct

**Step 4: Run cargo tests**

```bash
cargo test
```

Expected: All tests pass.

**Step 5: Run clippy**

```bash
cargo clippy -- -D warnings
```

Expected: No warnings.

**Step 6: Commit any fixes, then push**

```bash
git push
```

---

### Task 6: Cleanup

**Goal:** Remove any remaining dead code and verify CI passes.

**Step 1: Check for unused code**

Verify these are no longer referenced and can be cleaned up:
- `titlecase()` in `src/lib.rs` — check if still used (was used for chart titles, may not be needed with JSON output)
- `Titles` struct — already removed in Task 1
- Any `use plotly` statements — should all be gone

**Step 2: Monitor CI**

After pushing, verify:
- The `Publish docs` workflow runs and deploys successfully
- The site is live at the GitHub Pages URL

**Step 3: Clean up the `book/` directory if it exists locally**

```bash
rm -rf book/
echo "book/" >> .gitignore
```
