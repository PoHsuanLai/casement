# dev/hw

`smoke.sh` is the person's optional real-hardware run: it records KMS, input and GPU fixtures
that the headless fakes replay. It refuses without a terminal and a typed confirmation. Agents
never run it, and no gate depends on it (20-FREEZE-PLAN §3.2, C3). It is written by lane P1-D2.
