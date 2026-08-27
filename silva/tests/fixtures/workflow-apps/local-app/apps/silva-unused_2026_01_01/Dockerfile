# No job references silva-unused:2026_01_01, so this must never be built.
# It fails on purpose: if the referenced-only filter ever regresses, the
# build breaks loudly instead of passing quietly.
FROM ubuntu:latest
RUN echo "this app is referenced by no job and must never be built" && exit 7
