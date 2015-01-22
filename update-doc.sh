#!/bin/sh
DIR=$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )

DOC_PATH="$DIR/earth-docs"

cargo doc --no-deps

if [ -d $DOC_PATH ]; then
    if [ -d "$DOC_PATH/.git" ]; then
        pushd $DOC_PATH
        git checkout gh-pages
        git pull
    else
        echo "Error: $DOC_PATH is not a git repository!"
        exit -1;
    fi
else
    git clone -b gh-pages git@github.com:earthreader/rust-earth.git $DOC_PATH
    pushd $DOC_PATH
fi
git rm -rf nightly/*
cp -R ../target/doc/* nightly/
git add nightly/*
git commit -m "Update nightly doc" --signoff
git push -f origin gh-pages
popd
