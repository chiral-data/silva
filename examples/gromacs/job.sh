#!/bin/bash
#
# This is an example from the Gromacs tutorial
# by Dr. Justin A. Lemku from Virginia Tech Department of Biochemistry
# http://www.mdtutorials.com/gmx/lysozyme/
#

echo 15 | gmx pdb2gmx -f 1AKI_clean.pdb -o 1AKI_processed.gro -water spce
ls /opt/artifact/
