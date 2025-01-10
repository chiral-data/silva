#!/bin/bash
#
# Benchmark example from
# https://ftp.gromacs.org/pub/benchmarks/
# 	water_GMX50_bare.tar.gz
#

GMX=/usr/local/gromacs/avx2_256/bin/gmx

$GMX grompp -f pme.mdp 
$GMX mdrun -noconfout -nsteps 1000 -v -pin on -nb gpu
