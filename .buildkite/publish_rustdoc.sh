#!/bin/bash

set -e
shopt -s globstar

docker run -w /build -v $(pwd):/build prismagraphql/build:test cargo rustdoc --features full,json-1,uuid-0_8,chrono-0_4,tracing-log,serde-support

rm -rf deploy_docs
git clone --branch gh-pages "git@github.com:prisma/quaint.git" deploy_docs > /dev/null 2>&1
rm -rf deploy_docs/*
cd deploy_docs

git config user.name "Buildkite agent"
git config user.email "hello@prisma.io"

mv ../target/doc/* .
echo "<meta http-equiv=refresh content=0;url=quaint/index.html>" > index.html

DIFF=$(git status -s)
printf "$DIFF\n"

if [ -z "$DIFF" ]
then
    echo "Nothing to do"
else
    git add -A .
    git commit -m "[skip ci] Rebuild pages at ${BUILDKITE_COMMIT}"

    echo
    echo "Pushing docs..."
    git push origin gh-pages
    echo
    echo "Docs published."
    echo
fi
