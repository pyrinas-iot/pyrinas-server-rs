# 0.2.2 - 3/7/2021

* Removes all usage of the UnixSocket
* Renamed Sock to Admin
* File and Host entries changed to Option since they're not used at the CLI level (this shouldn't affect server side or firmware but should be tested..)
* Separating example code placing them within the specific related library
* Fixed intermittent bug in OTA tests due to tests run in parallel
* Reorganizing tasks as there are a few "Optional" tasks (like influx and the admin console)
* Tasks now get tasks specific settings instead of PyrinasSettings