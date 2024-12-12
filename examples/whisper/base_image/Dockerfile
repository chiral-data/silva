# Reference: https://zenn.dev/kento1109/articles/d7d8f512802935

FROM pytorch/pytorch:latest

RUN apt-get update && apt-get install -y \
    build-essential \
    gcc \
    git \
    ffmpeg \
    && rm -rf /var/lib/apt/lists/*

RUN pip install -U pip

RUN pip install -U whisper 
