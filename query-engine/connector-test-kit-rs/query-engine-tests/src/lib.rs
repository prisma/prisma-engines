// Rules for writing tests:
// - mod name + test name have to be unique in name across all test suites.
// - tests must be annotated with `connector_test`
// - test modules can be annotated with `test_suite`. you get some niceties like imports and the ability to define
// - you can use ONE OF `only` or `exclude` to scope connectors.
//    - if you use none, the test is valid for all connectors.
//
// Notes:
// - Allow dead code should be set?
// - Tests run in separate units in the data source. For MySQL, this may be a separate database, for postgres a schema, etc. -> These units are named `{mod_name}_{test_name}`
// - Test logs. We could write the logs for each test into a logs folder with a file named after the test.

pub mod schemas;
