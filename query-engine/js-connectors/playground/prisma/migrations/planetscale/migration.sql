CREATE TABLE type_test (
    id INT AUTO_INCREMENT PRIMARY KEY,
    --
    -- ColumnType::Int32, PlanetScale::{INT8, INT16, INT24, INT32}
    tinyint_column TINYINT NOT NULL,
    tinyint_column_null TINYINT,
    smallint_column SMALLINT NOT NULL,
    smallint_column_null SMALLINT,
    mediumint_column MEDIUMINT NOT NULL,
    mediumint_column_null MEDIUMINT,
    int_column INT NOT NULL,
    int_column_null INT,
    --
    -- ColumnType::Int64, PlanetScale::{INT64}
    bigint_column BIGINT NOT NULL,
    bigint_column_null BIGINT,
    --
    -- ColumnType::Float, PlanetScale::{FLOAT32}
    float_column FLOAT(10, 2) NOT NULL,
    float_column_null FLOAT(10, 2),
    --
    -- ColumnType::Double, PlanetScale::{FLOAT64}
    double_column DOUBLE(15, 4) NOT NULL,
    double_column_null DOUBLE(15, 4),
    --
    -- ColumnType::Numeric, PlanetScale::{DECIMAL}
    decimal_column DECIMAL(10, 2) NOT NULL,
    decimal_column_null DECIMAL(10, 2),
    --
    -- ColumnType::Boolean, PlanetScale::{BOOL}
    boolean_column BOOLEAN NOT NULL,
    boolean_column_null BOOLEAN,
    --
    -- ColumnType::Bit, PlanetScale::{BIT}
    bit_column BIT NOT NULL,
    bit_column_null BIT,
    --
    -- ColumnType::Char
    char_column CHAR(10) NOT NULL,
    char_column_null CHAR(10),
    --
    -- ColumnType::Text, PlanetScale::{TEXT, VARCHAR}
    varchar_column VARCHAR(255) NOT NULL,
    varchar_column_null VARCHAR(255),
    text_column TEXT NOT NULL,
    text_column_null TEXT,
    --
    -- ColumnType::Date, PlanetScale::{DATE}
    date_column DATE NOT NULL,
    date_column_null DATE,
    --
    -- ColumnType::Time, PlanetScale::{TIME}
    time_column TIME NOT NULL,
    time_column_null TIME,
    --
    -- ColumnType::Year, PlanetScale::{YEAR}
    year_column YEAR NOT NULL,
    year_column_null YEAR,    
    --
    -- ColumnType::DateTime, PlanetScale::{DATETIME}
    datetime_column DATETIME NOT NULL,
    datetime_column_null DATETIME,
    --
    -- ColumnType::Timestamp, PlanetScale::{TIMESTAMP}
    timestamp_column TIMESTAMP NOT NULL,
    timestamp_column_null TIMESTAMP,
    --
    -- ColumnType::JSON, PlanetScale::{JSON}
    json_column JSON NOT NULL,
    json_column_null JSON,
    --
    -- ColumnType::Enum, PlanetScale::{ENUM}
    enum_column ENUM('value1', 'value2', 'value3') NOT NULL,
    enum_column_null ENUM('value1', 'value2', 'value3'),
    --
    -- ColumnType::Binary, PlanetScale::{BINARY}
    binary_column BINARY(64) NOT NULL,
    binary_column_null BINARY(64),
    --
    -- ColumnType::VarBinary, PlanetScale::{VARBINARY}
    varbinary_column VARBINARY(128) NOT NULL,
    varbinary_column_null VARBINARY(128),
    --
    -- ColumnType::VarBinary, PlanetScale::{VARBINARY}
    blob_column BLOB NOT NULL,
    blob_null BLOB,
    --
    -- ColumnType::Set, PlanetScale::{SET}
    set_column SET('option1', 'option2', 'option3') NOT NULL,
    set_column_null SET('option1', 'option2', 'option3')
);

INSERT INTO type_test (
    tinyint_column,
    smallint_column,
    mediumint_column,
    int_column,
    bigint_column,
    float_column,
    double_column,
    decimal_column,
    boolean_column,
    bit_column,
    char_column,
    varchar_column,
    text_column,
    date_column,
    time_column,
    year_column,
    datetime_column,
    timestamp_column,
    json_column,
    enum_column,
    binary_column,
    varbinary_column,
    blob_column,
    set_column
) VALUES (
    127, -- tinyint
    32767, -- smallint
    8388607, -- mediumint
    2147483647, -- int
    9223372036854775807, -- bigint
    3.402823466, -- float
    1.7976931348623157, -- double
    99999999.99, -- decimal
    TRUE, -- boolean
    1, -- bit
    'c', -- char
    'Sample varchar', -- varchar
    'This is a long text...', -- text
    '2023-07-24', -- date
    '23:59:59', -- time
    2023, -- year
    '2023-07-24 23:59:59', -- datetime
    '2023-07-24 23:59:59', -- timestamp
    '{"key": "value"}', -- json
    'value3', -- enum
    0x4D7953514C, -- binary
    0x48656C6C6F20, -- varbinary
    _binary 'binary', -- blob
    'option1,option3' -- set
);
