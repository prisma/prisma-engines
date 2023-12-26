#!/usr/bin/env node
import { spawnSync  } from 'node:child_process';
import { createWriteStream  } from 'node:fs'

function escape(s) {
    let lookup = {
        '&': "&amp;",
        '"': "&quot;",
        '\'': "&apos;",
        '<': "&lt;",
        '>': "&gt;"
    };
    return s.replace( /[&"'<>]/g, c => lookup[c] );
}

const twiggyOut = spawnSync('twiggy',  ['dominators', '-f', 'json', 'pkg/query_engine_bg.wasm'], {
    encoding: 'utf8',
    maxBuffer: Infinity
}).stdout
const data = JSON.parse(twiggyOut)

const file = createWriteStream('result.html')
const bytesFmt = Intl.NumberFormat("en", {
    notation: "compact",
    style: "unit",
    unit: "byte",
    unitDisplay: "narrow",
  });

function printTree(items) {
    for (const item of items)  {
        const hasChildren = Boolean(item.children && item.children.length > 0)
        if (hasChildren) {
            const open = item.retained_size > 1024 * 10 ? 'open' : ''
            file.write(`<details ${open} class="item"><summary>`)
        } else {
            file.write(`<div class="item">`)
        }
        const sizeBytes = bytesFmt.format(item.retained_size)
        const sizePercent = item.retained_size_percent.toFixed(2)
        file.write(`<span class="size">${sizePercent}%, ${sizeBytes}</span> ${escape(item.name)}`)
        if (hasChildren) {
            file.write(`</summary>`)
            printTree(item.children)
            file.write('</details>')
        } else {
            file.write('</div>')
        }
    }
}
file.write('<!DOCTYPE html>')
file.write('<html>')
file.write('<head>')
file.write(`
<style>
body {
    font-size: 16px;
    font-family: monospace;
}
.item {
    margin-left: 1em;
}
.size {
    font-weight: bold;
}
</style>`)
file.write('<head>')
file.write('<body>')
printTree(data.items)
file.write('</body>')
file.write('</html>')
file.close()