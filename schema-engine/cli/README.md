# schema-engine

The schema-engine CLI binary.

## Usage

```
schema-engine-cli d24a48807efc126453d67d38031205cffb90a268
When no subcommand is specified, the schema engine will default to starting as a JSON-RPC server over stdio

USAGE:
    schema-engine [OPTIONS] --datasource <JSON> [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -d, --datamodels <FILE>...                 List of paths to the Prisma schema files
        --datasource <JSON>                    Optional JSON string to override the `datasource` block's URLs in the
                                               schema. This is derived from a Prisma Config file with `engines:
                                               'classic'`
    -e, --extension-types <extension-types>

SUBCOMMANDS:
    cli     Doesn't start a server, but allows running specific commands against Prisma
    help    Prints this message or the help of the given subcommand(s)
```

### `cli` subcommand

```
schema-engine-cli 0.1.0
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
