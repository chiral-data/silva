#!/bin/bash
#
# This is an example from the Gromacs tutorial
# by Dr. Justin A. Lemku from Virginia Tech Department of Biochemistry
# http://www.mdtutorials.com/gmx/lysozyme/
#
# input_files:
# 1AKI_clean.pdb
#
# outputs_files:
# porse.itp


echo 15 | gmx pdb2gmx -f 1AKI_clean.pdb -o 1AKI_processed.gro -water spce
# cp 1AKI_processed.gro /opt/artifact/
# cp posre.itp /opt/artifact/
# cp topol.top /opt/artifact/
ls /opt/artifact/
