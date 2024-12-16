# Reference: https://zenn.dev/kento1109/articles/d7d8f512802935

FROM pytorch/pytorch:latest

RUN apt-get update && apt-get install -y \
    build-essential \
    gcc \
    git \
    ffmpeg \
    && rm -rf /var/lib/apt/lists/*

RUN pip install -U pip
RUN pip install git+https://github.com/openai/whisper.git 

RUN mkdir -p /opt/models
ADD https://openaipublic.azureedge.net/main/whisper/models/aff26ae408abcba5fbf8813c21e62b0941638c5f6eebfb145be0c9839262a19a/large-v3-turbo.pt /opt/models/large-v3-turbo.pt
