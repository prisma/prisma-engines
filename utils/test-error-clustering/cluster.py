import re
import sys
import json
from collections import defaultdict
from dataclasses import dataclass
from enum import Enum
from typing import List, Dict, Optional, Iterable

import matplotlib.pyplot as plt
from mpl_toolkits.mplot3d import Axes3D
import numpy as np
from sklearn.feature_extraction.text import TfidfVectorizer
from sklearn.manifold import TSNE
from sklearn.cluster import DBSCAN


@dataclass
class ClusteringOptions:
    max_representative_outputs: int = 3  # Show at most this many representative outputs per cluster
    manifold_dimension: int = 2  # Dimension of the clustering space, target dimension of the t-SNE algorithm
    dbscan_epsilon: float = 3.0  # Higher value produces less clusters
    show_cleaned_stdout: bool = True  # Helps debugging the cleanup method
    plot_clusters: bool = True  # Plots the clusters, works only if manifold_dimension is 2 or 3


@dataclass
class TestResult:
    name: str
    output: str
    clean_output: str


COMMON_LINES = (
    '<SEP>',
    'Used datamodel:',
    'Some details are omitted',
    'Test failed due to an error',
    'stack backtrace:',
    'at <PATH>:<DEC>:<DEC>',
    'at .<PATH>:<DEC>:<DEC>',
    'request; method="initializeSchema" params',
    '[<TIMESTAMP> INFO tokio_postgres::connection] NOTICE:',
)

RX_BLOCK_START = re.compile(
    r'''^(
        datasource\s+\w+\s*\{
        |generator\s+\w+\s*\{
        |model\s+\w+\s*\{
    )$''',
    re.VERBOSE
)
assert RX_BLOCK_START.match("datasource test {")
assert RX_BLOCK_START.match("datasource test{")
assert RX_BLOCK_START.match("generator client {")
assert RX_BLOCK_START.match("generator client{")
assert RX_BLOCK_START.match("model User {")
assert RX_BLOCK_START.match("model User{")

RX_PANIC_START = re.compile(r"^(thread '[\w:]+' panicked at .*?<PATH>:<DEC>:<DEC>|<DEC>: rust_begin_unwind).*$")
assert RX_PANIC_START.match(
    r"thread 'writes::top_level_mutations::non_embedded_upsert::non_embedded_upsert::nested_delete_in_update' panicked at query-engine<PATH>:<DEC>:<DEC>:")


class Mode(Enum):
    keep = 'keep'
    star = 'star'
    panic = 'panic'
    block = 'block'


def clean_doc(lines: Iterable[str]) -> Iterable[str]:
    mode: Mode = Mode.keep
    for line in lines:
        line = clean_line(line)

        if not line or any(common in line for common in COMMON_LINES):
            continue

        if mode == Mode.star:
            if line.startswith('*'):
                continue
            else:
                mode = Mode.keep

        if mode == Mode.panic:
            if line.startswith('<DEC>: '):
                continue
            else:
                mode = Mode.keep

        if mode == Mode.block:
            if line == '}':
                mode = Mode.keep
            continue

        if line == '* Test run information:':
            mode = Mode.star
        elif RX_PANIC_START.match(line):
            mode = Mode.panic
        elif RX_BLOCK_START.match(line):
            mode = Mode.block
        else:
            yield line


def clean_line(line: str) -> str:
    line = re.sub(r'\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z?', '<TIMESTAMP>', line)  # Timestamps
    line = re.sub(r'\d{4}-\d{2}-\d{2}', '<DATE>', line)  # Timestamps
    line = re.sub(r'\d{2}:\d{2}:\d{2}', '<TIME>', line)  # Timestamps
    line = re.sub(r'/[\w/.-]+', '<PATH>', line)  # Unix file paths
    line = re.sub(r'\\[\w\\.-]+', '<PATH>', line)  # Windows file paths
    line = re.sub(r'\b[0-9a-fA-F\-]{8,}\b', '<ID>', line)  # IDs
    line = re.sub(r'\b[0-9a-fA-F]{8,}\b', '<HEX>', line)  # Hex numbers, addresses
    line = re.sub(r'\b\d+\b', '<DEC>', line)  # Decimal numbers
    line = re.sub(r'\b[+\-]?\d+\.\d+([eE][+\-]?\d+)?\b', '<FLOAT>', line)  # Floating point numbers
    line = re.sub(r'={3,}', '<SEP>', line)  # Separators
    line = re.sub(r'\*{3,}', '<SEP>', line)  # Separators
    line = re.sub(r'-{3,}', '<SEP>', line)  # Separators
    line = re.sub(r'[ \t]{2,}', ' ', line)  # Repeated spaces and tabs, but not newlines
    return line.strip()


def plot_clusters_2d(reduced: np.ndarray, labels: np.ndarray):
    for label in set(labels):
        if label < 0:
            continue
        mask = labels == label
        cluster_label = f"Cluster {label}"
        plt.scatter(
            reduced[mask, 0],
            reduced[mask, 1],
            label=cluster_label,
            alpha=0.6
        )

    plt.xlabel('X')
    plt.ylabel('Y')


def plot_clusters_3d(reduced: np.ndarray, labels: np.ndarray):
    assert Axes3D, '3D projection requires Axes3D to be imported'
    ax = plt.axes(projection='3d')

    for label in set(labels):
        if label < 0:
            continue
        mask = labels == label
        cluster_label = f"Cluster {label}"
        ax.scatter(
            reduced[mask, 0],
            reduced[mask, 1],
            reduced[mask, 2],
            label=cluster_label,
            alpha=0.6
        )

    ax.set_xlabel('X')
    ax.set_ylabel('Y')
    # noinspection PyUnresolvedReferences
    ax.set_zlabel('Z')


class TestFailureClustering:
    def __init__(self, options: ClusteringOptions):
        self.options = options
        self.tests: List[TestResult] = []
        self.clusters: Dict[int, list[TestResult]] = defaultdict(list)
        self.not_clustered: int = 0

        assert self.options.max_representative_outputs >= 1
        assert self.options.manifold_dimension >= 2
        assert self.options.dbscan_epsilon > 0

    def run(self, jsonl_path: str):
        self.read_test_results(jsonl_path)

        plot_path = f'{jsonl_path}.png' if self.options.plot_clusters else None
        self.cluster_test_results(plot_path)

        md_path = f'{jsonl_path}.md'
        self.write_markdown(md_path)

        fail_path = f'{jsonl_path}.fail'
        self.write_failed_tests(fail_path)

    def read_test_results(self, jsonl_path: str):
        with open(jsonl_path, 'rt', encoding='utf-8') as f:
            for line in f:
                line = line.strip()
                if not line.startswith('{') or not line.endswith('}'):
                    continue

                try:
                    obj = json.loads(line)
                except json.decoder.JSONDecodeError:
                    continue

                if obj.get('event') != 'failed':
                    continue

                name = obj.get('name')
                if not name:
                    continue

                stdout = obj.get('stdout')
                if not stdout:
                    continue

                name = name.split('$')[-1].split('#')[0]
                cleaned_stdout = '\n'.join(clean_doc(stdout.split('\n')))

                cleaned_stdout = cleaned_stdout.replace(name, '<TESTCASE>')

                if not cleaned_stdout.strip():
                    print(f'WARNING: Test output got completely removed by the cleanup: {name}')
                    continue

                self.tests.append(TestResult(
                    name=name,
                    output=stdout,
                    clean_output=cleaned_stdout,
                ))

    def cluster_test_results(self, plot_path: Optional[str] = None):
        self.clusters.clear()

        count = len(self.tests)
        if count < 3:
            for i, test in enumerate(self.tests):
                self.clusters[1 + i].append(test)
            return

        # Vectorize the test outputs based on term frequencies
        vectorizer = TfidfVectorizer(analyzer=lambda doc: doc.strip().splitlines(), max_features=8192)
        documents = [test.clean_output for test in self.tests]
        vectors = vectorizer.fit_transform(documents).toarray()
        count, terms = vectors.shape

        # Reduce to 2 dimensions using t-SNE
        tsne = TSNE(n_components=self.options.manifold_dimension, random_state=42, perplexity=min(count - 1, 50))
        reduced = tsne.fit_transform(vectors)

        # Cluster using DBSCAN
        dbscan = DBSCAN(eps=self.options.dbscan_epsilon, min_samples=2)
        labels = dbscan.fit_predict(reduced)
        self.not_clustered = np.sum(labels == -1)

        # Organize tests by clusters
        for label, test in zip(labels, self.tests):
            if label < 0:
                # Ignore noisy samples (see the doc of dbscan.fit_predict),
                # these test errors are rare, therefore they are too much
                # outside the other clusters. They should be handled later.
                continue
            self.clusters[label].append(test)

        # Plot the clusters
        self.plot_clusters(plot_path, reduced, labels)

    def plot_clusters(self, plot_path: Optional[str], reduced: np.ndarray, labels: np.ndarray):
        if not plot_path or self.options.manifold_dimension not in (2, 3):
            return

        plt.figure(figsize=(16, 16))

        if self.options.manifold_dimension == 2:
            plot_clusters_2d(reduced, labels)
        else:
            plot_clusters_3d(reduced, labels)

        plt.title("Test error clustering (t-SNE + DBSCAN)")
        plt.tight_layout()
        plt.savefig(plot_path)
        plt.close()

    def write_markdown(self, md_path: str):
        # Iterate the clusters in decreasing order of test counts
        sorted_clusters = sorted(self.clusters.items(), key=lambda x: len(x[1]), reverse=True)
        with open(md_path, 'wt', encoding='utf-8') as f:
            for cluster_no, (cluster_id, tests) in enumerate(sorted_clusters, 1):

                # Format the cluster as Markdown
                print(f"# Cluster {cluster_no} ({len(tests)} tests)", file=f)
                print(file=f)
                for name in sorted(test.name for test in tests):
                    print(f'- {name}', file=f)
                print(file=f)

                if self.options.max_representative_outputs < 1:
                    continue

                # Sort the tests inside the cluster by frequency of the normalized output,
                test_groups = defaultdict(list)
                for test in tests:
                    test_groups[test.clean_output].append(test)
                sorted_docs = sorted(test_groups, key=lambda d: len(test_groups[d]), reverse=True)

                # Provide the top N representative log outputs as collapsed blocks
                for i, doc in enumerate(sorted_docs[:self.options.max_representative_outputs]):
                    test_group = test_groups[doc]
                    test = test_group[0]
                    print('<details>', file=f)
                    print(f"<summary>{1 + i}: {test.name} ({len(test_group)})</summary>", file=f)
                    print(file=f)
                    if self.options.show_cleaned_stdout:
                        print('```js', file=f)
                        print(test.clean_output.replace('```', '`~`~`'), file=f)
                        print('```', file=f)
                        print(file=f)
                        print('<details>', file=f)
                        print('<summary>Full log</summary>', file=f)
                        print(file=f)
                    print('```js', file=f)
                    print(test.output.replace('```', '`~`~`'), file=f)
                    print('```', file=f)
                    print(file=f)
                    if self.options.show_cleaned_stdout:
                        print('</details>', file=f)
                        print(file=f)
                    print('</details>', file=f)
                    print(file=f)
                print(file=f)

    def write_failed_tests(self, fail_path):
        with open(fail_path, 'wt', encoding='utf-8') as f:
            for name in sorted(set(test.name for test in self.tests)):
                print(name, file=f)


def main():
    if len(sys.argv) < 2:
        print(f'Usage: {sys.argv[0]} <test-results.jsonl>')
        sys.exit(1)

    jsonl_path = sys.argv[1]

    options = ClusteringOptions()
    clustering = TestFailureClustering(options)
    clustering.run(jsonl_path)

    test_count = len(clustering.tests)
    cluster_count = len(clustering.clusters)
    not_clustered = clustering.not_clustered
    not_clustered_pct = 100.0 * not_clustered / test_count

    print(f'Failed tests: {test_count}')
    print(f'Clusters: {cluster_count}')
    print(f'Not clustered: {not_clustered} ({not_clustered_pct:.2f}%)')


if __name__ == "__main__":
    main()
