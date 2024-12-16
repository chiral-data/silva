import whisper

model = whisper.load_model("turbo", download_root="/opt/models")
result = model.transcribe("alisa_fujii_sample_2.mp3")
print(result["text"])
