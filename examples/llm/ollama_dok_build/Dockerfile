FROM ollama/ollama

ADD ./.ollama /root/.ollama
RUN mkdir -p /opt/llm_ollama
# ENV OLLAMA_MODELS=/opt/llm_ollama
ADD entrypoint.sh /opt/llm_ollama/entrypoint.sh
WORKDIR /opt/llm_ollama

ENTRYPOINT ["/bin/bash"]
CMD ["entrypoint.sh"]

