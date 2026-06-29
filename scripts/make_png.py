from PIL import Image
import sys

def convert(path, out):
    img = Image.open(path)
    img.save(out)

convert('/home/parobek/.gemini/antigravity-cli/brain/a26a6c4d-7308-48db-a8e6-25e3cd18e612/starfox_frame_1080_fix.ppm', '/home/parobek/.gemini/antigravity-cli/brain/a26a6c4d-7308-48db-a8e6-25e3cd18e612/starfox_frame_1080_test.png')
