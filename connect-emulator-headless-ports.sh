#!/bin/bash -v
set -e
# map ports over ssh to connect to remote emulator
ssh -N -L 5554:localhost:5554 -L 5555:localhost:5555 $1
