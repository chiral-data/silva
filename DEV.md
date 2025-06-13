# For Developers

## Release

### Toolchain for realease (currently BROKEN!!!)
The release process relies on [cargo-dist](https://github.com/axodotdev/cargo-dist), configured via `dist-workspace.toml`.

Unfortunately, the original cargo-dist repository is not actively maintained. This, combined with GitHub Actions [deprecating Ubuntu 20.04](https://github.com/actions/runner-images/issues/11101), has broken the release workflow.

[A fork](https://github.com/astral-sh/cargo-dist) of the original repository is being maintained. A new solution based on this updated fork is being investigated. 


### How to create a release
- git commit -am "release: 0.2.0"
- git tag "v0.2.0"
- git push
- git push --tags
