Hand-written topologies in the shape `dev/hw/smoke.sh` records (JSON of `KmsResources`). Replace
them with real recordings when the owner has run the smoke on a machine. `dev/check-kms-fixtures.sh`
checks the shape; the FakeKms suite replays every file here.
