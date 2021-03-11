# Changelog

All notable changes to this project will be documented in this file. This file adheres to the format of [keep a changelog.](https://keepachangelog.com/en/1.0.0/)

## [0.2.3] - 3/10/2021

### Added
* Detailed documentation and the skeleton of further detailed how-to in the `docs` folder.
* Github configuration in `.github`

### Changed
* Moved items out of `lib-shared` that aren't necessary. This reduces build time of `pyrinas-cli` significantly (~250+ packages down to ~115)
* Updated to `influxdb` 0.4.0

## [0.2.2] - 3/7/2021

### Changed
* Renamed Sock to Admin
*  File and Host entries changed to Option since they're not used at the CLI level (this shouldn't affect server side or firmware but should be tested..)
* Separating example code placing them within the specific related library
* Fixed intermittent bug in OTA tests due to tests run in parallel
* Reorganizing tasks as there are a few "Optional" tasks (like influx and the admin console)
* Tasks now get tasks specific settings instead of PyrinasSettings

### Removed
* Removes all usage of the UnixSocket







