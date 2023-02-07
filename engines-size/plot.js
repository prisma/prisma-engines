import * as d3 from "https://cdn.jsdelivr.net/npm/d3@7/+esm";
import * as Plot from "https://cdn.jsdelivr.net/npm/@observablehq/plot@0.6/+esm";

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

function createPlot({ data, yDomainFromZero = false }) {
  const yDomain = [
    yDomainFromZero ? 0 : d3.min(data, (d) => d.size_bytes),
    d3.max(data, (d) => d.size_bytes),
  ];

  return Plot.plot({
    width: document.body.clientWidth,
    marginLeft: 70,
    marginRight: 170,
    x: {
      grid: true,
    },
    y: {
      domain: yDomain,
      grid: true,
      tickFormat: formatMB,
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

function formatMB(bytes) {
  const megabytes = bytes / 1024 / 1024;
  const rounded = Math.round(megabytes * 100) / 100;
  return `${rounded} MB`;
}
