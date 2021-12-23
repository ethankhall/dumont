#!/bin/bash

set -eux

http POST localhost:3030/api/org org=example
http GET localhost:3030/api/org
for REPO in {1..10}; do
    http POST localhost:3030/api/org/example/repo repo=example$REPO
done
http GET localhost:3030/api/org/example/repo
