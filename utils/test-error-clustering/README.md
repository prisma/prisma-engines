# Clustering test errors

## Dependencies

Tested on Python 3.12.3. 

```shell
pip install -r requirements.txt
```

## Gathering test output

Prepare the test environment specific to your target. For example:
```shell
make dev-sqlite
```

Run the Query Engine tests using Nextest to produce `libtest-json` output.
It is an experimental feature of Nextest at the time of writing, which needs
to be enabled by the `NEXTEST_EXPERIMENTAL_LIBTEST_JSON` environment variable.

Nextest will generate data on its `stdout` in JSONL format, e.g. one JSON
object each line. Each object includes the result of a test case and its
captured `stdout` and `stderr` in case the test failed.

```shell
export NEXTEST_EXPERIMENTAL_LIBTEST_JSON=1
cargo nextest run \
  --test-threads=1 \
  --package query-engine-tests \
  --message-format libtest-json \
  >"test.jsonl" 2>"test.log"
```

The above command will run one test at a time, which may take very long.

You can parallelize the tests with `--test-threads=8`

Running the tests in parallel may fail with `XFAIL` or `EXECFAIL` errors.
In this case run multiple processes instead, then concatenate the resulting
JSONL files. With `SHARD` 1..8: `--test-thread=1 --partition hash:"$SHARD/8"`

## Clustering the test failures

```shell
python cluster.py test.jsonl
```

The script will write these files into the same folder as the input file is in:

- `test.jsonl.md`: Markdown formatted clustering with representative test output
- `test.jsonl.png`: Cluster visualization, mainly useful for debugging
- `test.jsonl.fail`: List of failed test cases, useful to update a known failure list

The `fail` list is sorted and contains each failed test only once (unique).

If there are no failed tests, empty `md` and `fail` files will be written. If there
are less than 3 test failures, then no actual clustering will happen and no PNG
will be produced, but the output will still contain all the failed tests.

## Remarks

The goal is not to have perfect clustering, but has good enough automated
clustering to be able to target the most frequent issues first. Is it recommended
to re-run all tests and repeat the clustering after fixing a few issues.

The same test may show up in multiple clusters, if it fails with different errors
due to failed retries. In order to avoid confusion and to speed up testing it is
recommended to set `retries = 0` in `.config/nextest.toml` to disable retries.

The script has some hardcoded heuristics to determine the `perplexity` parameter 
of the t-SNE algorithm used. It may need to be changed/tuned in the future if
the clustering is not good enough.