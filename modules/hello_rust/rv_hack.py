#!/usr/bin/env python3
import sys
import json

'''
this script modifies target spec (.json)
'''
spec_name = sys.argv[1]
mode = sys.argv[2] # relocation-model: static/pic
with open(spec_name, 'r+') as f:
    spec = json.load(f)
    spec['relocation-model'] = mode
    f.seek(0)
    f.truncate()
    json.dump(spec, f, indent=2)
    f.write('\n')
