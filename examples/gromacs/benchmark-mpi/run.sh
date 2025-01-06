#!/bin/bash
#
# a free benchmark set of Gromacs
# by Max Planck Institute
#

GMX=/usr/local/gromacs/avx2_256/bin/gmx
BENCHMARK=cmet_eq

# $GMX mdrun -s $BENCHMARK.tpr -nsteps 10000 -ntomp 2
$GMX mdrun -s $BENCHMARK.tpr -nsteps 1000 

# Benchmark result
# Cloud provider: Sakura Internet
# V100 result 14.339 ns/day
# H100 result
#                                 steps       ns/day    core t(s)   wall t(s)
# V100-DOK    1-MPI,3-OpenMP      1,000       14.339    36.181      12.063
# V100-DOK    1-MPI,4-OpenMP      1,000       13.558    51.020      12.758 
# V100-DOK    1-MPI,2-OpenMP      1,000       14.524    23.814      11.910 
# V100-DOK    1-MPI,3-OpenMP     10,000       20.447 
# V100-DOK    1-MPI,4-OpenMP     10,000       18.266
# V100-DOK    1-MPI,2-OpenMP     10,000       20.655
# V100-DOK    1-MPI,3-OpenMP    100,000       23.567 
# V100-Server 1-MPI,4-OpenMP      1,000       51.143    13.528       3.382
# V100-Server 1-MPI,3-OpenMP      1,000       41.236    12.584       4.195 
# V100-Server 1-MPI,2-OpenMP      1,000       24.351    14.205       7.103 
# V100-Server 1-MPI,2-OpenMP     10,000       27.727   124.653      62.329
# V100-Server 1-MPI,3-OpenMP     10,000       38.862   133.405      44.470 
# V100-Server 1-MPI,4-OpenMP     10,000       51.118 
# H100-DOK    1-MPI,10-OpenMP   100,000      105.692
# H100-DOK    1-MPI,10-OpenMP 1,000,000      108.374
#
