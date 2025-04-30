import sys
import json
import re
from collections import defaultdict, Counter
from dataclasses import dataclass
from math import ceil, sqrt
from typing import List, Dict, Optional

import matplotlib.pyplot as plt
import numpy as np
from sklearn.feature_extraction.text import TfidfVectorizer
from sklearn.manifold import TSNE
from sklearn.cluster import DBSCAN


@dataclass
class ClusteringOptions:
    representative_output_count: int = 1  # How many representative outputs to show per cluster
    dbscan_epsilon: float = 3.0  # Higher value produces less clusters
    log_cleaned_stdout: bool = False  # Helps debugging the cleanup method
    plot_clusters: bool = True


@dataclass
class TestResult:
    name: str
    output: str
    clean_output: str


class TestFailureClustering:
    def __init__(self, options: ClusteringOptions):
        self.options = options
        self.tests: List[TestResult] = []
        self.clusters: Dict[int, list[TestResult]] = defaultdict(list)

    def clean_test_output(self, text: str) -> str:
        """Cleans dynamic parts like timestamps, IDs, file paths from the test output"""
        text = re.sub(r'\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}', '<TIMESTAMP>', text)  # Timestamps
        text = re.sub(r'\d{4}-\d{2}-\d{2}', '<DATE>', text)  # Timestamps
        text = re.sub(r'\d{2}:\d{2}:\d{2}', '<TIME>', text)  # Timestamps
        text = re.sub(r'/[\w/.-]+', '<PATH>', text)  # Unix file paths
        text = re.sub(r'\\[\w\\.-]+', '<PATH>', text)  # Windows file paths
        text = re.sub(r'[0-9a-fA-F\-]{36}', '<UID>', text)  # UIDs
        text = re.sub(r'[0-9a-fA-F]{4,}', '<HEX>', text)  # Hex numbers, addresses
        text = re.sub(r':\d+', ':<LINE>', text)  # Line numbers
        text = re.sub(r'\d{2,}', '<DEC>', text)  # Decimal numbers
        text = re.sub(r'[+\-]?\d+\.\d+', '<FLOAT>', text)  # Floating point numbers
        text = re.sub(r'={3,}', '===', text)  # Separators
        text = re.sub(r'\*{3,}', '***', text)  # Separators
        text = re.sub(r'-{3,}', '---', text)  # Separators
        text = re.sub(r'[ \t]{2,}', ' ', text)  # Repeated spaces and tabs, but not newlines
        text = re.sub(r'\n{2,}', '\n', text)  # Repeated newlines
        return text.strip()

    def run(self, jsonl_path: str):
        self.read_test_results(jsonl_path)

        plot_path = f'{jsonl_path}.png' if self.options.plot_clusters else None
        self.cluster_test_results(plot_path)

        md_path = f'{jsonl_path}.md'
        self.write_markdown(md_path)

        fail_path = f'{jsonl_path}.fail'
        self.write_failed_tests(fail_path)

    def read_test_results(self, jsonl_path: str):
        with open(jsonl_path, 'r', encoding='utf-8') as f:
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
                cleaned_stdout = self.clean_test_output(stdout)

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
        vectorizer = TfidfVectorizer(max_features=8192)
        vectors = vectorizer.fit_transform(test.clean_output for test in self.tests)

        # Reduce to 2 dimensions using t-SNE
        tsne = TSNE(n_components=2, random_state=42, perplexity=int(ceil(sqrt(count))))
        reduced = tsne.fit_transform(vectors.toarray())

        # Cluster using DBSCAN
        dbscan = DBSCAN(eps=self.options.dbscan_epsilon, min_samples=2)
        labels = dbscan.fit_predict(reduced)

        # Organize tests by clusters
        for label, test in zip(labels, self.tests):
            if label < 0:
                # Ignore noisy sample (see the doc of dbscan.fit_predict)
                continue
            self.clusters[label].append(test)

        # Plot the clusters
        self.plot_clusters(plot_path, reduced, labels)

    def plot_clusters(self, plot_path: Optional[str], reduced: np.ndarray, labels: np.ndarray):
        if not plot_path:
            return

        plt.figure(figsize=(12, 8))
        for label in set(labels):
            if label < 0:
                continue
            mask = labels == label
            plt.scatter(reduced[mask, 0], reduced[mask, 1], label=f"Cluster {label}", alpha=0.6)

        plt.title("Test error clustering (t-SNE + DBSCAN)")
        plt.savefig(plot_path)

    def write_markdown(self, md_path: str):
        # Iterate the clusters in decreasing order of test counts
        sorted_clusters = sorted(self.clusters.items(), key=lambda x: len(x[1]), reverse=True)
        with open(md_path, 'w', encoding='utf-8') as f:
            for cluster_no, (cluster_id, tests) in enumerate(sorted_clusters, 1):

                # Format the cluster as Markdown
                print(f"# Cluster {cluster_no} ({len(tests)} tests)", file=f)
                print(file=f)
                for name in sorted(test.name for test in tests):
                    print(f'- {name}', file=f)
                print(file=f)

                if self.options.representative_output_count < 1:
                    continue

                # Find the most common test output inside the cluster
                frequencies = Counter(t.clean_output for t in tests)
                top_n = [doc for doc, _ in frequencies.most_common(self.options.representative_output_count)]
                original_outputs = {t.clean_output: t.output for t in tests}

                # Provide the top N representative log outputs as collapsed blocks
                for i, cleaned_doc in enumerate(top_n):
                    print('<details>', file=f)
                    print('<summary>', file=f)
                    print(f"Representative output {1 + i}", file=f)
                    print('</summary>', file=f)
                    print(file=f)
                    print('```js', file=f)
                    print(original_outputs[cleaned_doc].replace('```', '`~`~`'), file=f)
                    print('```', file=f)
                    print(file=f)
                    if self.options.log_cleaned_stdout:
                        print(f'### Cleaned', file=f)
                        print('```js', file=f)
                        print(cleaned_doc.replace('```', '`~`~`'), file=f)
                        print('```', file=f)
                        print(file=f)
                    print('</details>', file=f)
                    print(file=f)
                print(file=f)

    def write_failed_tests(self, fail_path):
        with open(fail_path, 'w') as f:
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


if __name__ == "__main__":
    main()
