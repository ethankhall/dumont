# Dumont

[Dumont][1], the tower guardian, is a service for managing versions and the state
they are in.

## Example

Kevin creates a new version `1.6.9` of `clu` is created to fix the phone bill problems. Before Kevin
sends `clu` out on his mission, he'll want to know what version was built and attach a version
to git hash. Since only one `clu` can be on mission at a time, Kevin would update `1.6.9`'s labels
to denote that it was deployed and when.

In the mean time, Kevin goes and adds more improvements to `clu` and build version `1.7.1`. When
the day ends, Kevin goes home thinking that `1.7.1` is on a mission. When Kevin arrives back at work,
he notices that `clu` gave an error that he thought he fixed. Instead of digging into the code, Kevin
asked `dumont` what version was deployed, realized the mistake and send `1.7.1` out to fix the phone
bill.

## Features

- Normal github style organization. (org/repo/version)
- Policy enforcement for required labels on repos and versions.
- Multiple policies can be applied based on org/repo names.
- Postgresql backend.
- Tested.

## Non-Features

- Authentication. Authentication is hard, and best done by another application like nginx. See [this blog](https://fardog.io/blog/2017/12/30/client-side-certificate-authentication-with-nginx/) post for how to do mutual auth (x509) with nginx.
- Authorization. This is a feature that could be added in the future, but is currently missing.

  [1]: https://tron.fandom.com/wiki/Dumont