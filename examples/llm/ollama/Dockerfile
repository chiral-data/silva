FROM ollama/ollama

RUN mkdir -p /opt/llm_ollama
ADD ./run.sh /opt/llm_ollama

WORKDIR /opt/llm_ollama
ENTRYPOINT ["/bin/bash"]
CMD ["run.sh"]
