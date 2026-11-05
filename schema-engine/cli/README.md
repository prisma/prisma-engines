# schema-engine

The schema-engine CLI binary.

## Usage

```
When no subcommand is specified, the schema engine will default to starting as a JSON-RPC server over stdio

USAGE:
    schema-engine [OPTIONS] --datasource <JSON> [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -d, --datamodels <FILE>...                 List of paths to the Prisma schema files
        --datasource <JSON>                    JSON connection configuration for the schema engine.
                                               The Prisma CLI obtains these URLs from `prisma.config.ts`
                                               datasource configuration.
    -e, --extension-types <extension-types>

SUBCOMMANDS:
    cli     Doesn't start a server, but allows running specific commands against Prisma
    help    Prints this message or the help of the given subcommand(s)
```

This is an interface summary. Current Prisma Config supplies `datasource.url`
and, when configured, `datasource.shadowDatabaseUrl`; it has no
`engine: 'classic'` or `engines: 'classic'` option. The native CLI's older help
comment may still mention that removed configuration. JSON passed through
`--datasource` uses `url` and `shadowDatabaseUrl` keys; it does not put URLs
back into the PSL datasource block.

### `cli` subcommand

```
Doesn't start a server, but allows running specific commands against Prisma

USAGE:
    schema-engine cli <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    can-connect-to-database    Does the database connection string work?
    create-database            Create an empty database defined in the configuration string
    drop-database              Drop the database
    help                       Prints this message or the help of the given subcommand(s)
```
