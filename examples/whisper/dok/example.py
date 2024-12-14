import whisper

model = whisper.load_model("turbo", download_root="/opt/models")
result = model.transcribe("japanese.mp3")
print(result["text"])
