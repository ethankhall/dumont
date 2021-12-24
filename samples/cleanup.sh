#!/bin/bash

set -eux


for REPO in {1..10}; do
    http DELETE localhost:3030/api/org/example/repo/example$REPO
done
http DELETE localhost:3030/api/org/example
