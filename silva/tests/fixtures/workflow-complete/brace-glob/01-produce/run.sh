#!/bin/sh
echo ">seq1" > seq.fasta
echo "ACGT" >> seq.fasta
echo ">ref1" > ref.fa
echo "TTGG" >> ref.fa
echo ">prot1" > proteins.faa
echo "MKVL" >> proteins.faa
echo "this should not be copied" > notes.txt
