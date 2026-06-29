from PIL import Image
import sys

def check(path):
    img = Image.open(path)
    w, h = img.size
    
    # Check left vs right half
    left_non_black = 0
    right_non_black = 0
    
    for y in range(h):
        for x in range(w):
            pixel = img.getpixel((x, y))
            # handle both RGB and RGBA
            if isinstance(pixel, int):
                if pixel != 0:
                    if x < w // 2: left_non_black += 1
                    else: right_non_black += 1
            elif pixel[:3] != (0, 0, 0):
                if x < w // 2:
                    left_non_black += 1
                else:
                    right_non_black += 1
                    
    print(f"{path} -> Left: {left_non_black}, Right: {right_non_black}")

check('/home/parobek/.gemini/antigravity-cli/brain/a26a6c4d-7308-48db-a8e6-25e3cd18e612/starfox_frame_1080.png')
check('/home/parobek/.gemini/antigravity-cli/brain/a26a6c4d-7308-48db-a8e6-25e3cd18e612/starfox_frame_1200.png')
check('/home/parobek/.gemini/antigravity-cli/brain/a26a6c4d-7308-48db-a8e6-25e3cd18e612/starfox_frame_1320.png')
check('/home/parobek/.gemini/antigravity-cli/brain/a26a6c4d-7308-48db-a8e6-25e3cd18e612/starfox_frame_1560.png')
