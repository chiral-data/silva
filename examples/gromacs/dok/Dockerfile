FROM nvcr.io/hpc/gromacs:2023.2

RUN mkdir -p /opt/gromacs_dok
ADD ./1AKI_clean.pdb /opt/gromacs_dok
ADD ./job.sh /opt/gromacs_dok
ADD ./run.sh /opt/gromacs_dok

WORKDIR /opt/gromacs_dok
CMD ["sh", "run.sh"]
