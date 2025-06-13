# For Developers

## Release

### Toolchain for realease (currently BROKEN!!!)
[cargo-dist](https://github.com/axodotdev/cargo-dist) is used for release.
The configuration file is dist-workspace.toml.

The update of the original repo is suspending and [the depreccate of ubuntu 20.04 from github action](https://github.com/actions/runner-images/issues/11101) broken the release workflow.
There exists [this fork](https://github.com/astral-sh/cargo-dist) keeping the development, a new solution based on this fork has to be investigated.


### How to create a release
- git commit -am "release: 0.2.0"
- git tag "v0.2.0"
- git push
- git push --tags
