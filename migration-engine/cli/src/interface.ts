interface StdErrLine {
    timestamp: string
    level: LogLevel
    fields: LogFields
}

interface LogFields {
    message: string

    // Only for ERROR level messages
    is_panic?: boolean
    error_code?: string

    [key: string]: any
}

type LogLevel = "INFO" | "ERROR" | "DEBUG" | "WARN"
