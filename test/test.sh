#!/bin/bash

make clean && rm -fr eyec-report.json && PATH=../bin/:$PATH make
node ../src/graph.js eyec-report.json | dot -Tsvg > output.svg && eog output.svg

