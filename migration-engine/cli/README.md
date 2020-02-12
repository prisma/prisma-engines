# migration-engine

The migration-engine CLI binary.

## Usage

```
migration-engine-cli 0.1.0
When no subcommand is specified, the migration engine will default to starting as a JSON-RPC server over stdio.

USAGE:
    migration-engine [FLAGS] [OPTIONS] [SUBCOMMAND]

FLAGS:
    -h, --help          Prints help information
    -s, --single_cmd    Run only a single command, then exit
        --version       Prints the server commit ID

OPTIONS:
    -d, --datamodel <FILE>    Path to the datamodel

SUBCOMMANDS:
    cli     Doesn't start a server, but allows running specific commands against Prisma
    help    Prints this message or the help of the given subcommand(s)
```

### `cli` subcommand

```
migration-engine-cli 0.1.0
Doesn't start a server, but allows running specific commands against Prisma

USAGE:
    migration-engine cli --datasource <datasource> <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -d, --datasource <datasource>    The connection string to the database

SUBCOMMANDS:
    --can_connect_to_database    Does the database connection string work?
    --create_database            Create an empty database defined in the configuration string
    help                         Prints this message or the help of the given subcommand(s)
```
