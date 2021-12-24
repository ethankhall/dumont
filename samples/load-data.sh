#!/bin/bash

set -eux

http POST localhost:3030/api/org org=example
http GET localhost:3030/api/org
for REPO in {1..10}; do
    http POST localhost:3030/api/org/example/repo repo=example$REPO labels:="{\"scm_url\": \"https://github.com/example/example$REPO\"}"
done

http GET localhost:3030/api/org/example/repo

for REPO in {1..5}; do
    http PUT localhost:3030/api/org/example/repo/example$REPO labels:="{\"scm_url\": \"https://example$REPO\"}"
done

http GET localhost:3030/api/org/example/repo