#!/bin/bash

http POST localhost:3030/model/abc-123

cat<<EOF | http PATCH localhost:3030/model/abc-123/sub-1
{
    "ops": [
            { "op": "test", "path": "/a/b/c", "value": "foo" },
            { "op": "remove", "path": "/a/b/c" },
            { "op": "add", "path": "/a/b/c", "value": [ "foo", "bar" ] },
            { "op": "replace", "path": "/a/b/c", "value": 42 },
            { "op": "move", "from": "/a/b/c", "path": "/a/b/d" },
            { "op": "copy", "from": "/a/b/d", "path": "/a/b/e" }
    ],
    "source": "app1",
    "actor": "ethan",
    "effectiveTime": 12345,
    "traceId": 123
}
EOF