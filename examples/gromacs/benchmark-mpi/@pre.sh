#!/bin/bash
#
# This is a script to be executed before the job completion
#

PREFIX=3925268
BENCHMARK=cmet_eq
# PREFIX=3925290
# BENCHMARK=hif2a_eq

echo "preprocessing"
curl -O https://www.mpinat.mpg.de/$PREFIX/$BENCHMARK.zip 
unzip $BENCHMARK.zip
