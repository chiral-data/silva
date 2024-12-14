FROM nvcr.io/hpc/gromacs:2023.2

RUN mkdir -p /opt//home/qw/dev/silva/examples/gromacs_dok
ADD ./1AKI_clean.pdb /opt//home/qw/dev/silva/examples/gromacs_dok
ADD ./job.sh /opt//home/qw/dev/silva/examples/gromacs_dok
ADD ./run.sh /opt//home/qw/dev/silva/examples/gromacs_dok

WORKDIR /opt//home/qw/dev/silva/examples/gromacs_dok
CMD ["sh", "run.sh"]
