# Dumont API

This document will use [httpie](https://github.com/httpie/httpie) commands. The command
is called `http`.

## API Response Structure

In general the API response follows the following scheme.

```json
{
    "data": {
        ...
    },
    "status": {
        "code": 200,
        "errors": []
    },
    "page": {
        "more": false,
        "total": 100
    }
}
```

The field `.status.code` will also the HTTP response code, but it's often easier to use
the filed in JSON when writing scripts, so it's included.
The field `.data` may be absent, if there is no data to return. This only ever happens when there is an error.
The field `.status.errors` may be absent, in the case there are no errors.
The field `.page` may be absent, if the response is only a single object.
The field `.page.more` declares if there are more pages to fetch.
The field `.page.total` declares the total number of objects avaliable.

## Organization
### Create Organization

To create an organization POST against `/api/org`.

```
> jq -n '{ "org": "example" }' | http POST localhost:3030/api/org
HTTP/1.1 200 OK
content-length: 48
content-type: application/json
date: Thu, 30 Dec 2021 18:41:48 GMT

{
    "data": {
        "org": "example"
    },
    "status": {
        "code": 200
    }
}
```

### List Organizations

To list the known organizations, execute a GET against `/api/org`

```
> http GET localhost:3030/api/org
HTTP/1.1 200 OK
content-length: 50
content-type: application/json
date: Thu, 30 Dec 2021 18:42:45 GMT

{
    "data": [
        {
            "org": "example"
        }
    ],
    "status": {
        "code": 200
    }
}
```

### Get an Organization

To get a known organization, execute a GET against `/api/org/{org name}`

```
> http GET localhost:3030/api/org/example
HTTP/1.1 200 OK
content-length: 48
content-type: application/json
date: Thu, 30 Dec 2021 18:45:08 GMT

{
    "data": {
        "org": "example"
    },
    "status": {
        "code": 200
    }
}
```

### Delete an Organization

To delete an organizations, execute a DELETE against `/api/org/{org name}`.
This will fail if there are any repos existing under the organization.

```
> http DELETE  localhost:3030/api/org/example
HTTP/1.1 200 OK
content-length: 47
content-type: application/json
date: Thu, 30 Dec 2021 18:50:26 GMT

{
    "data": {
        "deleted": true
    },
    "status": {
        "code": 200
    }
}
```

An example of trying to delete an org with repositories under it. The error message
will change in the future.
```
> http DELETE localhost:3030/api/org/example
HTTP/1.1 500 Internal Server Error
content-length: 208
content-type: application/json
date: Thu, 30 Dec 2021 18:50:54 GMT

{
    "status": {
        "code": 500,
        "error": [
            "Execution Error: error returned from database: update or delete on table \"organization\" violates foreign key constraint \"repository_org_id_fkey\" on table \"repository\""
        ]
    }
}
```

## Repository

These examples will assume that the organization exists.

### Create a Repository

```
> jq -n '{ "repo": "example-repo", "labels": { "owners": "bobby tables"} }' | http POST localhost:3030/api/org/example/repo
HTTP/1.1 200 OK
content-length: 105
content-type: application/json
date: Thu, 30 Dec 2021 18:54:54 GMT

{
    "data": {
        "labels": {
            "owners": "bobby tables"
        },
        "org": "example",
        "repo": "example-repo"
    },
    "status": {
        "code": 200
    }
}
```

Taking a look at the JSON sent to the API

```json
{
  "repo": "example-repo",
  "labels": {
    "owners": "bobby tables"
  }
}
```

`.repo` is required, and needs to be a string.
`.labels` is optional, but may be required based on the configuration of the server.

### List Repositories

Execute a `GET /api/org/example/repo`. There are two parameters that can be used
to page thought the API `size` and `page`. `size` defaults to `50` and `page` defaults to `0`.

```
> http GET localhost:3030/api/org/example/repo?page=0&size=50
HTTP/1.1 200 OK
content-length: 107
content-type: application/json
date: Thu, 30 Dec 2021 18:57:03 GMT

{
    "data": [
        {
            "labels": {
                "owners": "bobby tables"
            },
            "org": "example",
            "repo": "example-repo"
        }
    ],
    "status": {
        "code": 200
    }
}
```

### Get Repository

```
> http GET localhost:3030/api/org/example/repo/example-repo
HTTP/1.1 200 OK
content-length: 105
content-type: application/json
date: Thu, 30 Dec 2021 19:01:13 GMT

{
    "data": {
        "labels": {
            "owners": "bobby tables"
        },
        "org": "example",
        "repo": "example-repo"
    },
    "status": {
        "code": 200
    }
}
```

### Delete Repository

```
> http DELETE localhost:3030/api/org/example/repo/example-repo
HTTP/1.1 200 OK
content-length: 47
content-type: application/json
date: Thu, 30 Dec 2021 19:01:39 GMT

{
    "data": {
        "deleted": true
    },
    "status": {
        "code": 200
    }
}
```

### Update Repository

Updating a repository can only update labels. You cannot change anything else about it.
When updating the repository, the labels submitted will be replaced, fully. So there is no
partial update. To remove a single label the entire object must be reposed with the label missing.

```
> jq -n '{ "labels": { "owners": "bobby tables", "status": "deprecated"} }' | http PUT localhost:3030/api/org/example/repo/example-repo
HTTP/1.1 200 OK
content-length: 127
content-type: application/json
date: Thu, 30 Dec 2021 19:03:04 GMT

{
    "data": {
        "labels": {
            "owners": "bobby tables",
            "status": "deprecated"
        },
        "org": "example",
        "repo": "example-repo"
    },
    "status": {
        "code": 200
    }
}
```

The instance used for this example requires that a repo have a label `owners`. If an update is
applied that removes that label, the API will respond with

```
> jq -n '{ "labels": { "status": "deprecated"} }' | http PUT localhost:3030/api/org/example/repo/example-repo
HTTP/1.1 400 Bad Request
content-length: 138
content-type: application/json
date: Thu, 30 Dec 2021 19:03:52 GMT

{
    "status": {
        "code": 400,
        "error": [
            "Policy `library` required that label `owners` be set, however it was not and no default was specified."
        ]
    }
}
```

## Versions

Versions have the same API pattern that Repositories do.

### Create Version

This example shows an instance of "default" labels. `release_state` was automatically added
to the labels when the version was created.

```
> jq -n '{ "version": "1.2.3", "labels": { "git_hash": "9e7ae4f618358144ed35dc8b978cb8a75a85b99c"} }' | http POST localhost:3030/api/org/example/repo/example-repo/version
HTTP/1.1 200 OK
content-length: 142
content-type: application/json
date: Thu, 30 Dec 2021 19:07:47 GMT

{
    "data": {
        "labels": {
            "git_hash": "9e7ae4f618358144ed35dc8b978cb8a75a85b99c",
            "release_state": "released"
        },
        "version": "1.2.3"
    },
    "status": {
        "code": 200
    }
}
```

### List Versions

There are two parameters that can be used to page thought the API `size` and `page`.
`size` defaults to `50` and `page` defaults to `0`.

```
> http GET localhost:3030/api/org/example/repo/example-repo/version
HTTP/1.1 200 OK
content-length: 144
content-type: application/json
date: Thu, 30 Dec 2021 19:08:54 GMT

{
    "data": [
        {
            "labels": {
                "git_hash": "9e7ae4f618358144ed35dc8b978cb8a75a85b99c",
                "release_state": "released"
            },
            "version": "1.2.3"
        }
    ],
    "status": {
        "code": 200
    }
}
```

### Get Version

```
> http GET localhost:3030/api/org/example/repo/example-repo/version/1.2.3
HTTP/1.1 200 OK
content-length: 142
content-type: application/json
date: Thu, 30 Dec 2021 19:18:41 GMT

{
    "data": {
        "labels": {
            "git_hash": "9e7ae4f618358144ed35dc8b978cb8a75a85b99c",
            "release_state": "released"
        },
        "version": "1.2.3"
    },
    "status": {
        "code": 200
    }
}
```

### Update Version

```
> jq -n '{ "labels": { "git_hash": "9e7ae4f618358144ed35dc8b978cb8a75a85b99c", "release_state": "end-of-life"} }' | http PUT localhost:3030/api/org/example/repo/example-repo/version/1.2.3
HTTP/1.1 200 OK
content-length: 145
content-type: application/json
date: Thu, 30 Dec 2021 19:20:54 GMT

{
    "data": {
        "labels": {
            "git_hash": "9e7ae4f618358144ed35dc8b978cb8a75a85b99c",
            "release_state": "end-of-life"
        },
        "version": "1.2.3"
    },
    "status": {
        "code": 200
    }
}
```

### Delete Version

```
> http DELETE localhost:3030/api/org/example/repo/example-repo/version/1.2.3
HTTP/1.1 200 OK
content-length: 47
content-type: application/json
date: Thu, 30 Dec 2021 19:21:18 GMT

{
    "data": {
        "deleted": true
    },
    "status": {
        "code": 200
    }
}
```
