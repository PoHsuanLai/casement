# dev/hw

`smoke.sh` is the person's optional real-hardware run: it reads what this machine's display,
input and GPU stacks report (sysfs, `drm_info`, `lspci`, `libinput list-devices`) into a
directory that a lane turns into the fixtures the headless fakes replay. It only reads. It
refuses without a terminal and a typed confirmation. Agents never run it, and no gate depends
on it (20-FREEZE-PLAN §3.2, C3). It is written by lane P1-D2.
