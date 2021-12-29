const fs = require('fs');
const { ChartJSNodeCanvas } = require('chartjs-node-canvas');

const width = 2048; //px
const height = 1080; //px
const canvasRenderService = new ChartJSNodeCanvas({ width, height, backgroundColour: "rgba(21,45,47,0.79)", chartCallback: (ChartJS) => {} });

(async () => {
    const benchmarkJson = JSON.parse(fs.readFileSync('../../data/benchmark.json', 'utf-8'));
    const sortedJson = benchmarkJson.sort((a, b) => a.createdAtTime - b.createdAtTime)
    const mapBenchmarks = sortedJson.map(item => item.benchmark);

    const buildChartConfig = (type) => ({
        type: type,
        data: {
            labels: sortedJson.map(item => item.createdAt),
            datasets: [{
                label: '3EM JS',
                data: mapBenchmarks.map(item => ((item.js || {}).mean || 0)),
                fill: false,
                borderColor: '#3d88ad',
                backgroundColor: '#3d88ad',
                tension: 0.1
            },
                {
                    label: '3EM WASM',
                    data: mapBenchmarks.map(item => ((item.wasm || {}).mean || 0)),
                    fill: false,
                    borderColor: 'rgb(3,22,68)',
                    backgroundColor: 'rgb(3,22,68)',
                    tension: 0.1
                },
                {
                    label: '3EM EVM',
                    data: mapBenchmarks.map(item => ((item.evm || {}).mean || 0)),
                    fill: false,
                    borderColor: 'rgb(42,218,187)',
                    backgroundColor: 'rgb(42,218,187)',
                    tension: 0.1
                },
                {
                    label: 'Redstone',
                    data: mapBenchmarks.map(item => ((item.redstoneJs || {}).mean || 0)),
                    fill: false,
                    borderColor: 'rgb(215,10,10)',
                    backgroundColor: 'rgb(215,10,10)',
                    tension: 0.1
                },
                {
                    label: 'Smartweave',
                    data: mapBenchmarks.map(item => ((item.smartweaveJs || {}).mean || 0)),
                    fill: false,
                    borderColor: 'rgb(103,102,102)',
                    backgroundColor: 'rgb(103,102,102)',
                    tension: 0.1
                }
            ]
        },
        options: {
            devicePixelRatio: 2,
            scales:{
                x: {
                    display: false
                }
            },
            legend: {
                labels: {
                    fontSize: 30,
                    fontColor: "white"
                }
            },
            responsive: false,
            maintainAspectRatio: false,
        }
    });

    const chartBar = await canvasRenderService.renderToBuffer(buildChartConfig('bar'));
    const line = await canvasRenderService.renderToBuffer(buildChartConfig('line'));

    // Write image to file
    fs.writeFileSync('../../data/benchmark_bar.png', chartBar);
    fs.writeFileSync('../../data/benchmark_line.png', line);
})();
