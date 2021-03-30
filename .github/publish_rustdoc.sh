#!/usr/bin/env bash

set -e
shopt -s globstar

git config user.name "GitHub agent"
git config user.email "hello@prisma.io"

rm -rf deploy_docs
git clone --branch gh-pages "git@github.com:prisma/quaint.git" deploy_docs > /dev/null 2>&1
rm -rf deploy_docs/*
mv target/doc/* deploy_docs/
echo "<meta http-equiv=refresh content=0;url=quaint/index.html>" > deploy_docs/index.html

DIFF=$(git -C deploy_docs status -s)
printf "$DIFF\n"

if [ -z "$DIFF" ]
then
    echo "Nothing to do"
else
    git -C deploy_docs add -A .
    git -C deploy_docs commit -m "Add changes"
fi
