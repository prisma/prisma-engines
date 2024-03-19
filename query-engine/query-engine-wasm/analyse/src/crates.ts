import twiggyData from '../out/twiggy.profiling.json' with { type: 'json' }

type TwiggyEntry = {
  name: string // name of a symbol in a Wasm binary
  shallow_size: number // size as bytes
  shallow_size_percent: number // percentage as float
}

type ParsedTwiggyEntry = {
  crate: string
  original: TwiggyEntry
}

function parseEntry({ name, ...rest }: TwiggyEntry): ParsedTwiggyEntry | undefined {
  const sections = [
    'data',
    'type',
    'global',
    'table',
    'elem',
    'memory',
  ]
  
  if (
    sections.some(section => name.startsWith(`${section}[`))
  ) {
    let sectionName = name.split('[')[0]
    sectionName = `(section) ${sectionName}[..] `

    return {
      crate: sectionName,
      original: {
        name,
        ...rest,
      },
    }
  }
  
  const prefixesToAvoid = [        
    // exported functions
    'queryengine_',
    'getBuildTimeInfo',

    // compiler utilities (e.g., `__rust_alloc`, arithmetic routines, etc.)
    '__',

    // misc. noise
    '"function names"',
    'export ',
    'import ',
  ]

  const substringsToAvoid = [
    'section headers',
    'custom section',
    'wasm magic'
  ]

  const stringsToAvoid = [
    // memory utilities
    'memmove',
    'memset',
    'memcpy',
    'memcmp',
  ]
  
  if (
    prefixesToAvoid.some(prefix => name.startsWith(prefix))
    || substringsToAvoid.some(substring => name.includes(substring))
    || stringsToAvoid.includes(name)
  ) {
    return undefined
  }

  // Symbols like:
  // ```
  // <builtin_psl_connectors::mysql_datamodel_connector::MySqlDatamodelConnector
  //   as psl_core::datamodel_connector::Connector>::provider_name::h9720aa1dba9e87e9
  // >
  // ```
  // should be parsed as:
  // ```
  // psl_core::datamodel_connector::Connector
  // ```
  const match = name.match(/<.*? as (.*?)>::/)
  name = match ? match[1] : name

  // extract the crate name, e.g., `psl_core`
  const crateName = name.split('::')[0]

  return {
    crate: crateName,
    original: {
      name,
      ...rest,
    },
  }
}

// print the twiggy data as a markdown table with columns:
// - crate: the name of the crate
// - bytes: the number of bytes the crate occupies
// - frequency: the number of times the crate is referenced
function printAsMarkdown(twiggyMap: TwiggyMap, { CRATE_NAME_PADDING }: { CRATE_NAME_PADDING: number }) {
  const BYTE_SIZE_PADDING = 8
  const PERCENT_SIZE_PADDING = 8
  const FREQUENCY_PADDING = 10

  console.log(`| ${'crate'.padStart(CRATE_NAME_PADDING)} | ${'size(KB)'.padEnd(BYTE_SIZE_PADDING)} | ${'size(%)'.padEnd(PERCENT_SIZE_PADDING)} | ${'frequency'.padEnd(FREQUENCY_PADDING)} |`)
  console.log(`| ${'-'.repeat(CRATE_NAME_PADDING - 1)}: | :${'-'.repeat(BYTE_SIZE_PADDING - 1)} | :${'-'.repeat(PERCENT_SIZE_PADDING - 1)} | :${'-'.repeat(FREQUENCY_PADDING - 1)} |`)

  for (const [crate, { size, percent, entries }] of twiggyMap.entries()) {
    console.log(`| ${crate.padStart(CRATE_NAME_PADDING)} | ${size.toFixed(1).padStart(BYTE_SIZE_PADDING)} | ${(percent.toFixed(3)+"%").padStart(PERCENT_SIZE_PADDING) } | ${entries.length.toString().padStart(FREQUENCY_PADDING)} |`)
  }
}

type TwiggyMapValue = {
  size: number
  percent: number
  entries: ParsedTwiggyEntry[]
}

type TwiggyMap = Map<
  ParsedTwiggyEntry['crate'],
  TwiggyMapValue
>

function analyseDeps(twiggyData: TwiggyEntry[]): TwiggyMap {
  const BYTES_IN_KB = 1024.0
  // parse the twiggy data, filter out noise entries, and for each crate,
  // keep track of how much space it takes up and the twiggy entries that belong to it
  const twiggyMap = twiggyData
    .map(parseEntry)
    .filter((entry): entry is ParsedTwiggyEntry => entry !== undefined)
    .reduce((acc, item) => {
      const { crate, original } = item

      // get a reference to the current map entry for the crate, if it already exists
      const currEntry = acc.get(crate)

      if (currEntry) {
        currEntry.size += (original.shallow_size / BYTES_IN_KB)
        currEntry.percent += original.shallow_size_percent
        currEntry.entries.push(item)
      } else {
        acc.set(crate, {
          size: original.shallow_size / BYTES_IN_KB,
          percent: original.shallow_size_percent,
          entries: [item],
        })
      }

      return acc
    }, new Map() as TwiggyMap)

  // sort the map values by space occupied, descending
  // (maps maintain insertion order)
  const sortedTwiggyMap = new Map(
    [...twiggyMap.entries()].sort((a, b) => b[1].size - a[1].size)
  )

  return sortedTwiggyMap
}

function main() {
  const sortedTwiggyMap = analyseDeps(twiggyData as TwiggyEntry[])
  
  // visual adjustment for the "crate" column in the markdown table
  const CRATE_NAME_PADDING = 24

  printAsMarkdown(sortedTwiggyMap, {
    CRATE_NAME_PADDING,
  })
}

main()
