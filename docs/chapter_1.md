# Chapter 1

<div id="time_series_with_range_selector_buttons" class="plotly-graph-div" style="height:100%; width:100%;"></div>
<script type="text/javascript">
    Plotly.newPlot("time_series_with_range_selector_buttons", {
  "data": [
    {
      "type": "scatter",
      "mode": "lines+markers",
      "x": [
        "2024-03-08",
        "2024-03-09",
        "2024-03-10",
        "2024-03-11",
        "2024-03-12",
        "2024-03-13",
        "2024-03-14",
        "2024-03-15",
        "2024-03-16",
        "2024-03-17",
        "2024-03-18",
        "2024-03-19",
        "2024-03-20",
        "2024-03-21",
        "2024-03-22"
      ],
      "y": [
        "1631",
        "3788",
        "4586",
        "16412",
        "16801",
        "18913",
        "15468",
        "13340",
        "3546",
        "3855",
        "14104",
        "16260",
        "14723",
        "13658",
        "9796"
      ],
      "marker": {
        "size": 4,
        "line": {
          "width": 1.0,
          "color": "rgb(227, 61, 148)"
        },
        "color": "rgb(227, 61, 148)"
      }
    }
  ],
  "layout": {
    "title": {
      "text": "EKS Module Page Views"
    },
    "xaxis": {
      "title": {
        "text": "Date"
      },
      "rangeslider": {
        "visible": true
      },
      "rangeselector": {
        "buttons": [
          {
            "step": "month",
            "stepmode": "backward",
            "count": 1,
            "label": "1m"
          },
          {
            "step": "month",
            "stepmode": "backward",
            "count": 6,
            "label": "6m"
          },
          {
            "step": "year",
            "stepmode": "todate",
            "count": 1,
            "label": "YTD"
          },
          {
            "step": "year",
            "stepmode": "backward",
            "count": 1,
            "label": "1y"
          },
          {
            "step": "all"
          }
        ]
      }
    },
    "yaxis": {
      "title": {
        "text": "Views"
      }
    }
  },
  "config": {}
});
</script>
