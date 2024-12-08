FROM nvcr.io/hpc/gromacs:2023.2

RUN mkdir -p /opt/gromacs
ADD ./1AKI_clean.pdb /opt/gromacs

WORKDIR /opt/gromacs
ENTRYPOINT ["sh", "run.sh"]
