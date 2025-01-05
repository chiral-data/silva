#!/bin/bash
#
# This is a script to be executed before the job completion
#


echo "preprocessing"
curl -O https://files.rcsb.org/download/1AKI.pdb
grep -v HOH 1AKI.pdb > 1AKI_clean.pdb
