#!/usr/bin/env python3
import os
import time

# Set modification time to 15 years ago
old_time = time.time() - (15 * 365 * 24 * 60 * 60)
file_path = "test_photos/old_vacation_photos/old.jpg"

if os.path.exists(file_path):
    os.utime(file_path, (old_time, old_time))
    print(f"Set modification time of {file_path} to 15 years ago")
else:
    print(f"File {file_path} not found")