import os
base = "d:\\Programming\\1PRODUCTION\\Open Source\\tpt-gpu\\layer3_tptc"
for root, dirs, files in os.walk(base):
    for f in files:
        full = os.path.join(root, f)
        rel = os.path.relpath(full, base)
        print(rel)
