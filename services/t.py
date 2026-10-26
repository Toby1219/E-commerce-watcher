from datetime import datetime, timezone
import time

COOLDOWN = 3600

# Get the current timezone-aware UTC datetime object
utc_now = datetime.now(timezone.utc)

# Method 1: Convert to an ISO 8601 standard string (best for APIs, JSON, and databases)
iso_string = utc_now.isoformat()

# Method 2: Convert to a custom formatted string using strftime directives
custom_string = utc_now.strftime("%Y-%m-%d %H:%M:%S UTC")

# if time.monotonic() - utc_now < COOLDOWN:
print(True)
print(time.monotonic())
print(iso_string)
# print(time.monotonic() - iso_string)

# Print the resulting string representations to the console
print("ISO String:", iso_string)
print("Custom String:", custom_string)