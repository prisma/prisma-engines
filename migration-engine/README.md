# Prisma Migration Engine

This directory contains the crates that belong to the migration engine behind
[prisma-migrate](https://www.prisma.io/docs/concepts/components/prisma-migrate).

The code and documentation for the executable binary are in the [cli](./cli)
directory.

The core logic shared across connectors is in the [core](./core) directory.

The connector interface and the built-in connectors are in the
[connectors](./connectors) directory.

## API

The TypeScript CLI in [prisma/prisma](https://github.com/prisma/prisma)
interacts with the binary compiled from the [cli](./cli) crate. Some commands
are exposed through the CLI directly, like `create-database`, but most of them
through the JSON-RPC API, which is the default command if you just run the
`migration-engine` binary. These two sets of commands are separated because
previously, you needed a valid database connection to the database referenced
in the Prisma schema to start a migration engine instance, but we did not have
that requirement in commands like `migration-engine cli create-database`. This
is legacy, as the JSON-RPC API now connects lazily.

The reason why we have a JSON-RPC API in the first place is so the TypeScript
CLI can issue multiple commands on the same connection, get results back and
act on them.

Logging and crash reporting happens through JSON logs on the Migration Engine's
stderr. Every line contains a single JSON object conforming to the following
interface:

```typescript
interface StdErrLine {
  timestamp: string;
  level: LogLevel;
  fields: LogFields;
}

interface LogFields {
  message: string;

  /// Only for ERROR level messages
  is_panic?: boolean;
  error_code?: string;

  [key: string]: any;
}

type LogLevel = "INFO" | "ERROR" | "DEBUG" | "WARN";
```

### Exit codes

`0`: normal exit\
`1`: abnormal (error) exit\
`101`: panic

Non-zero exit codes should always be accompanied by a log message on stderr with
the `ERROR` level.
