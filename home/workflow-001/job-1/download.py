import io
import zipfile

import requests

FILE_URL = (
    "https://zenodo.org/records/17374422/files/constructed_library.zip?download=1"
)
# Extract to current directory since the zip already contains a folder
OUTPUT_DIR = "."

print(f"Downloading from Zenodo: {FILE_URL}")
response = requests.get(FILE_URL, stream=True)
response.raise_for_status()

# Get total file size for progress bar
total_size = int(response.headers.get("content-length", 0))
block_size = 8192

# Download with progress bar
# Configure tqdm for Docker logs: disable line-clearing, print new line for each update
downloaded_data = io.BytesIO()
bytes_downloaded = 0
chunk_count = 0
update_interval_mb = 0.2  # Print progress every 5 MB

print(f"Total size: {total_size / (1024 * 1024):.2f} MB")
for chunk in response.iter_content(chunk_size=block_size):
    if chunk:
        downloaded_data.write(chunk)
        bytes_downloaded += len(chunk)
        chunk_count += 1

        # Print progress every update_interval_mb
        if bytes_downloaded // (update_interval_mb * 1024 * 1024) > (
            bytes_downloaded - len(chunk)
        ) // (update_interval_mb * 1024 * 1024):
            progress_pct = (
                (bytes_downloaded / total_size) * 100 if total_size > 0 else 0
            )
            print(
                f"Progress: {bytes_downloaded / (1024 * 1024):.2f} MB / {total_size / (1024 * 1024):.2f} MB ({progress_pct:.1f}%)"
            )

# Final progress
print(f"Download complete: {bytes_downloaded / (1024 * 1024):.2f} MB (100%)")

print("Extracting files...")
downloaded_data.seek(0)
with zipfile.ZipFile(downloaded_data) as z:
    z.extractall(OUTPUT_DIR)

print(f"✅ Data successfully extracted to {OUTPUT_DIR}")
