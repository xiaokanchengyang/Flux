#!/usr/bin/env python3
"""
Create a default icon for Flux GUI
"""

from PIL import Image, ImageDraw, ImageFont
import os

# Create a 512x512 image with a nice gradient background
size = 512
img = Image.new('RGBA', (size, size), (0, 0, 0, 0))
draw = ImageDraw.Draw(img)

# Create gradient background
for y in range(size):
    # Gradient from dark blue to lighter blue
    r = int(20 + (y / size) * 30)
    g = int(40 + (y / size) * 60)
    b = int(120 + (y / size) * 80)
    draw.rectangle([(0, y), (size, y+1)], fill=(r, g, b, 255))

# Draw a stylized "F" for Flux
# Use a large, bold font-like shape
margin = size // 8
line_width = size // 8

# Vertical line of F
draw.rectangle(
    [(margin, margin), (margin + line_width, size - margin)],
    fill=(255, 255, 255, 255)
)

# Top horizontal line of F
draw.rectangle(
    [(margin, margin), (size - margin, margin + line_width)],
    fill=(255, 255, 255, 255)
)

# Middle horizontal line of F
middle_y = size // 2 - line_width // 2
draw.rectangle(
    [(margin, middle_y), (size - margin * 2, middle_y + line_width)],
    fill=(255, 255, 255, 255)
)

# Add subtle shadow effect
shadow_offset = 4
for rect in [
    [(margin + shadow_offset, margin + shadow_offset), 
     (margin + line_width + shadow_offset, size - margin + shadow_offset)],
    [(margin + shadow_offset, margin + shadow_offset), 
     (size - margin + shadow_offset, margin + line_width + shadow_offset)],
    [(margin + shadow_offset, middle_y + shadow_offset), 
     (size - margin * 2 + shadow_offset, middle_y + line_width + shadow_offset)]
]:
    draw.rectangle(rect, fill=(0, 0, 0, 50))

# Save the icon
output_path = os.path.join(os.path.dirname(__file__), '..', 'flux-gui', 'assets', 'icon.png')
os.makedirs(os.path.dirname(output_path), exist_ok=True)
img.save(output_path, 'PNG')
print(f"Created default icon at {output_path}")