import * as d3 from "https://cdn.jsdelivr.net/npm/d3@7/+esm";
import * as Plot from "https://cdn.jsdelivr.net/npm/@observablehq/plot@0.6/+esm";
import { Grid } from "https://cdn.jsdelivr.net/npm/ag-grid-community@29.1.0/+esm";

const data = d3.csvParse(
  await fetch("data.csv").then((response) => response.text()),
  (row) => ({
    ...row,
    date_time: new Date(row.date_time),
    size_bytes: Number(row.size_bytes),
  })
);

document.querySelector("#plot").append(
  createPlot({
    data,
    yDomainFromZero: true,
  })
);

document.querySelector("#plot-lib").append(
  createPlot({
    data: data.filter(({ file }) => file === "libquery_engine.node"),
  })
);

document.querySelector("#plot-bin").append(
  createPlot({
    data: data.filter(({ file }) => file === "query-engine"),
  })
);

document.querySelector("#plot-wasm").append(
  createPlot({
    data: data.filter(({ file }) => file === "query_engine_bg.wasm"),
  })
);

document.querySelector("#plot-wasm-gz").append(
  createPlot({
    data: data.filter(({ file }) => file === "query_engine_bg.wasm.gz"),
  })
);

function createPlot({ data, yDomainFromZero = false }) {
  const yDomain = [
    yDomainFromZero ? 0 : d3.min(data, (d) => d.size_bytes),
    d3.max(data, (d) => d.size_bytes),
  ];

  const digitsAfterComma = yDomainFromZero ? 2 : 3;

  return Plot.plot({
    width: document.body.clientWidth,
    marginLeft: 80,
    marginRight: 170,
    x: {
      grid: true,
    },
    y: {
      domain: yDomain,
      grid: true,
      tickFormat: (tick) => formatMB(tick, digitsAfterComma),
    },
    marks: [
      Plot.line(data, {
        x: "date_time",
        y: "size_bytes",
        stroke: "file",
      }),
      Plot.text(
        data,
        Plot.selectLast({
          x: "date_time",
          y: "size_bytes",
          z: "file",
          fill: "file",
          text: (d) => `${d.file} (${formatMB(d.size_bytes)})`,
          textAnchor: "start",
          dx: 2,
        })
      ),
    ],
  });
}

function formatMB(bytes, digitsAfterComma = 2) {
  const megabytes = bytes / 1024 / 1024;
  return `${megabytes.toFixed(digitsAfterComma)} MB`;
}

new Grid(document.querySelector("#grid"), {
  columnDefs: [
    { headerName: "Date and time", field: "date_time", sort: "desc" },
    { headerName: "Branch", field: "branch" },
    { headerName: "Commit", field: "commit" },
    { headerName: "File", field: "file" },
    { headerName: "Size (bytes)", field: "size_bytes" },
    {
      headerName: "Size (MB)",
      field: "size_bytes",
      valueFormatter: ({ value }) => formatMB(value),
    },
  ],
  rowData: data,
  defaultColDef: {
    sortable: true,
    filter: true,
    resizable: true,
  },
  enableCellTextSelection: true,
});
