#!/bin/bash

set -eux

http POST localhost:3030/api/orgs org=example
http GET localhost:3030/api/orgs
for REPO in {1..10}; do
    http POST localhost:3030/api/orgs/example/repos repo=example$REPO
done
http GET localhost:3030/api/orgs/example/repos

for REPO in {1..10}; do
    http GET localhost:3030/api/repos/example/example$REPO
done

for REPO in {1..10}; do
    http GET localhost:3030/api/repos/example/fake$REPO
done